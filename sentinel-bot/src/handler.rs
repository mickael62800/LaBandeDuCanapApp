//! EventHandler unifie — dispatche vers les modules.

use std::panic::AssertUnwindSafe;

use futures_util::FutureExt;
use serenity::async_trait;
use serenity::model::application::{
    CommandData, CommandDataOption, CommandDataOptionValue, Interaction,
};
use serenity::model::channel::{GuildChannel, Message};
use serenity::model::event::MessageUpdateEvent;
use serenity::model::gateway::Ready;
use serenity::model::guild::{Guild, Member, Role};
use serenity::model::id::{ChannelId, GuildId, MessageId, RoleId};
use serenity::model::user::User;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::grpc_client::{grpc_err_to_string, GrpcClientKey};
use crate::shared::heartbeat::{register_guilds, ApiClientKey};
use sentinel_proto::members::v1 as proto_members;

use crate::modules;

/// Retourne le module fonctionnel associe a une commande slash. Sert a
/// alimenter le champ `details.module` du log "command.invoked".
fn command_module(name: &str) -> &'static str {
    match name {
        "purge" | "cleanup" => "cleanup",
        "roles-panel" | "parrain" => "community",
        "audit" => "audit",
        "level" | "stats" | "progression-resync" | "classement" => "progression",
        "security" => "security",
        "automod" => "automod",
        "warn" | "unwarn" | "mute" | "unmute" | "ban" | "ban-sursis" | "unban" | "history"
        | "note" | "call" | "signalement" | "context" | "appeal" | "expirations" | "compare"
        | "modstats" | "evidence" | "review" | "template" | "transcript" | "export"
        | "massmute" | "massban" | "copilote" => "moderation",
        "ticket" | "ticket-admin" => "tickets",
        "idee" => "ideas",
        "confess" | "confess-admin" => "confessions",
        "backup" => "guild_backup",
        "logs-init" => "logs_setup",
        _ => "unknown",
    }
}

/// `true` si la commande est une commande admin/moderateur (loggue dans le
/// salon dedie `command_log_channel_id`). Couvre automod + toutes les commandes
/// de moderation/securite/nettoyage/audit, les `*-setup`, les `*-admin`
/// et les panneaux de config communautaires.
fn is_admin_command(name: &str) -> bool {
    // Les sanctions produisent DEJA une carte de moderation riche (cible,
    // moderateur, raison, strikes...) : on evite le doublon avec le log
    // une-ligne. Les commandes modo utilitaires (history, export, modstats...)
    // n'ont pas de carte -> elles restent loggees.
    if has_own_action_log(name) {
        return false;
    }
    matches!(
        command_module(name),
        "moderation" | "automod" | "security" | "cleanup" | "audit"
    ) || name.ends_with("-setup")
        || name.ends_with("-admin")
        || matches!(name, "roles-panel" | "parrain")
}

/// Commandes de sanction qui ont deja leur propre log riche (carte de
/// moderation) : exclues du log une-ligne pour eviter le doublon.
fn has_own_action_log(name: &str) -> bool {
    matches!(
        name,
        "warn" | "unwarn" | "mute" | "unmute" | "ban" | "unban" | "massmute" | "massban" | "note"
    )
}

/// Cherche (en descendant dans les sous-commandes) une option texte « reason »
/// / « raison » / « motif » pour l'ajouter au log de commande admin.
fn extract_reason(options: &[CommandDataOption]) -> Option<String> {
    for opt in options {
        match &opt.value {
            CommandDataOptionValue::String(s)
                if matches!(opt.name.as_str(), "reason" | "raison" | "motif")
                    && !s.trim().is_empty() =>
            {
                return Some(s.clone());
            }
            CommandDataOptionValue::SubCommand(sub)
            | CommandDataOptionValue::SubCommandGroup(sub) => {
                if let Some(r) = extract_reason(sub) {
                    return Some(r);
                }
            }
            _ => {}
        }
    }
    None
}

/// Reconstruit le nom complet de la commande slash y compris
/// subcommand_group / subcommand (ex: "ticket close all", "audit channel set").
fn format_full_command(data: &CommandData) -> String {
    let mut parts = vec![data.name.to_string()];
    fn descend(opts: &[CommandDataOption], parts: &mut Vec<String>) {
        for opt in opts {
            match &opt.value {
                CommandDataOptionValue::SubCommandGroup(sub_opts)
                | CommandDataOptionValue::SubCommand(sub_opts) => {
                    parts.push(opt.name.to_string());
                    descend(sub_opts, parts);
                }
                _ => {}
            }
        }
    }
    descend(&data.options, &mut parts);
    format!("/{}", parts.join(" "))
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(
            bot = %ready.user.name,
            guilds = ready.guilds.len(),
            "Sentinel Bot connecte"
        );

        register_guilds(&ctx, &ready).await;

        // Enregistrement per-guild des slash commands : filtre les modules
        // desactives via command_registry. Remplace l'ancien set_global_commands
        // qui enregistrait tout pour tout le monde -> impossible de cacher
        // une commande d'un module desactive.
        // On vide aussi les commandes globales heritees (set vide) car elles
        // sont visibles partout meme apres bascule per-guild.
        let _ =
            serenity::model::application::Command::set_global_commands(&ctx.http, Vec::new()).await;
        let guild_ids: Vec<_> = ready.guilds.iter().map(|g| g.id).collect();
        crate::command_registry::refresh_all_guilds(&ctx, &guild_ids).await;

        // ── Demarrage UNIQUE (une seule fois par process) ──
        // `ready()` refire a CHAQUE reconnexion Discord (et par shard). Tout ce
        // qui suit — tâches de fond, consumers Redis, hydratations de caches —
        // ne doit tourner qu'UNE fois : les caches en memoire persistent entre
        // reconnexions, et relancer les boucles = doublons (rappels postes
        // plusieurs fois, etc.). L'enregistrement des commandes ci-dessus, lui,
        // reste per-ready (idempotent, resync utile apres reconnexion).
        use std::sync::atomic::{AtomicBool, Ordering};
        static BOOTSTRAPPED: AtomicBool = AtomicBool::new(false);
        if BOOTSTRAPPED.swap(true, Ordering::SeqCst) {
            return;
        }

        // Panneau d'aide auto-genere : publie/maintient dans un salon le
        // catalogue de toutes les commandes (trie par categorie). Idempotent
        // (remplace ses anciens messages), n'affiche que les modules actifs.
        modules::help_panel::deploy_all(&ctx, ready.user.id, &guild_ids).await;

        // Listener Redis pour les events bot_enabled_changed -> re-register
        // les commandes guild a la volee quand un admin toggle on/off.
        crate::command_registry::spawn_consumer(ctx.clone());
        modules::community::load_temp_roles(&ctx, &guild_ids).await;
        modules::community::spawn_temp_role_cleanup(ctx.clone());

        modules::bump::spawn_background(ctx.clone());
        // Presence vocale : republication periodique. Sans elle, la page
        // membre perd un salon des que personne n'y bouge pendant trois
        // minutes — l'API considere alors l'instantane perime.
        modules::presence::spawn_background(ctx.clone());
        modules::nasa_apod::spawn_background(ctx.clone());

        // Security: sync membres au demarrage + background tasks
        let ctx_sec = ctx.clone();
        let guilds_for_sec: Vec<_> = ready.guilds.clone();
        tokio::spawn(async move {
            modules::security::on_ready_sync(&ctx_sec, &guilds_for_sec).await;
        });
        modules::security::spawn_background(ctx.clone());
        // Phase 5F — consumer Redis pour quarantine_expired (worker).
        modules::security::quarantine_expired_consumer::spawn(ctx.clone());
        // Phase 5G — consumer Redis pour lockdown_expired (worker).
        modules::security::lockdown_expired_consumer::spawn(ctx.clone());
        // Phase 5H — consumer Redis pour slowmode_expired (worker).
        modules::security::slowmode_expired_consumer::spawn(ctx.clone());

        // Sync periodique des roles Discord vers l'API (5 min)
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            modules::community::sync_all_guild_roles(&ctx_clone).await;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                modules::community::sync_all_guild_roles(&ctx_clone).await;
            }
        });

        // Audit: bootstrap watched users + Redis consumer
        modules::audit::on_ready(&ctx).await;

        // Automod: background tasks (slowmode deactivation + cache cleanup)
        modules::automod::spawn_background_tasks(&ctx);

        // Moderation: Redis consumer pour events externes
        modules::moderation::spawn_background(ctx.clone());

        // Voice: reconcile + spawn AFK sweep
        modules::voice::on_ready(&ctx, &ready).await;

        // Tickets: deploy panel + spawn background tasks (inactive close, SLA, Redis consumer)
        modules::tickets::on_ready(&ctx, &ready).await;
        modules::tickets::spawn_background(ctx.clone());

        // Idees: consumer des decisions prises depuis le web
        modules::ideas::on_ready(&ctx, &ready).await;
        modules::ideas::spawn_background(ctx.clone());

        // Progression: hydrate voice sessions + tick periodique credit XP
        modules::progression::on_ready(&ctx, &ready).await;
        modules::progression::spawn_voice_tick(ctx.clone());
        // Filet des paliers de roles : le level-up les applique deja sur
        // l'instant, cette boucle rattrape ce qu'aucun evenement ne signale
        // (role retire a la main, palier ajoute en configuration, bot arrete
        // pendant un level-up).
        modules::progression::role_tiers::spawn_verification_periodique(ctx.clone());

        // Announcements : consumer Redis stream pour les annonces planifiees
        // publiees par announcement-worker.
        modules::announcements::spawn(ctx.clone());
        modules::embeds::spawn(ctx.clone());
        modules::messages::spawn(ctx.clone());
        modules::cleanup::autopurge::spawn(ctx.clone());

        // Confessions : consumer Redis stream pour synchroniser les
        // suppressions web -> Discord (delete confession ou reply).
        modules::confessions::spawn_consumer(ctx.clone());

        // Welcome : consumer Redis pour publier le panneau de reglement
        // (bouton "Publier le reglement" du dashboard).
        modules::welcome::spawn(ctx.clone());

        // Guild backup : consumer Redis pour piloter capture/restore/wipe depuis
        // le web (events guild_backup:capture_requested / :restore_requested).
        modules::guild_backup::spawn(ctx.clone());

        // AI dataset : task de collecte (client-streaming gRPC longue duree).
        modules::ai_dataset::spawn_collector(ctx.clone()).await;
    }

    async fn message(&self, ctx: Context, msg: Message) {
        // On met en cache TOUS les messages (bots inclus) pour l'audit : ca
        // permet d'identifier une suppression de message de bot et de l'exclure
        // des logs. Le reste du pipeline ignore les bots.
        modules::audit::cache_message(&ctx, &msg).await;

        // Bump : la confirmation de /bump est postee par Disboard (un BOT). On
        // doit donc traiter ce module AVANT le filtre bot ci-dessous, sinon la
        // detection ne se declenche jamais. (Le module filtre lui-meme sur l'id
        // Disboard.)
        modules::bump::on_message(&ctx, &msg).await;

        if msg.author.bot {
            return;
        }

        // Activite ecrite affichee sur le site. Le module filtre lui-meme les
        // salons non visibles par @everyone.
        modules::presence::on_message(&ctx, &msg).await;

        // Salons "commandes uniquement" : supprime le message classique en
        // premier (avant l'XP / automod, qui n'ont pas a traiter un message
        // qui va disparaitre).
        if modules::command_channel::on_message(&ctx, &msg).await {
            return;
        }
        if modules::automod::on_message(&ctx, &msg).await {
            return;
        }
        modules::audit::on_message(&ctx, &msg).await;
        modules::progression::on_message(&ctx, &msg).await;
        modules::voice::on_message(&ctx, &msg).await;
        modules::tickets::on_message(&ctx, &msg).await;
        modules::ideas::on_message(&ctx, &msg).await;
        modules::ai_dataset::on_message(&ctx, &msg).await;
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        modules::audit::on_message_delete(&ctx, channel_id, message_id, guild_id).await;
    }

    async fn message_update(
        &self,
        ctx: Context,
        old: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        // Bump : DiscordL edite un message vide pour y mettre l'embed de
        // resultat -> on re-detecte a l'edition (avant le move de `event`).
        modules::bump::on_message_update(&ctx, &event).await;
        // Automod : re-analyse le contenu edite (contournement post-benin/edit).
        modules::automod::on_message_update(&ctx, &event).await;
        modules::audit::on_message_update(&ctx, old, new, event).await;
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        multiple_deleted: Vec<MessageId>,
        guild_id: Option<GuildId>,
    ) {
        modules::audit::on_message_delete_bulk(&ctx, channel_id, multiple_deleted, guild_id).await;
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        modules::audit::on_member_add(&ctx, &new_member).await;
        modules::welcome::on_member_add(&ctx, &new_member).await;
        modules::progression::assign_default_role(&ctx, &new_member).await;
        modules::community::on_member_add(&ctx, &new_member).await;
        modules::security::on_member_add(&ctx, &new_member).await;
        // Prefixe emoji staff : applique l'emoji des le (re)join si le membre
        // porte deja un role staff (guarde par staff_prefix_enabled, best-effort).
        modules::progression::nickname::on_member_add(&ctx, &new_member).await;
        // Guild backup : re-attribue les roles en attente d'un restore
        // (membres qui reviennent apres une restauration). Best-effort.
        modules::guild_backup::on_member_add(&ctx, &new_member).await;
        // Lifecycle : clear left_at + reset joined_at cote API. Le user
        // peut rejouer (wallet repart de zero, gere cote serveur).
        let guild_id = new_member.guild_id.to_string();
        let user_id = new_member.user.id.to_string();
        let grpc = ctx.data.read().await.get::<GrpcClientKey>().cloned();
        if let Some(grpc) = grpc {
            let req = proto_members::MemberLifecycleRequest { guild_id, user_id };
            if let Err(e) = crate::grpc_call!(@unit &grpc, members, rejoin_member, req) {
                warn!(error = %e, "Echec callback rejoin membre");
            }
        }
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        modules::audit::on_member_remove(&ctx, guild_id, &user).await;
        modules::welcome::on_member_remove(&ctx, guild_id, &user).await;
        modules::security::on_member_remove(&ctx, guild_id, &user).await;
        // Lifecycle : set left_at + reset wallet a 0. Le user n'apparaitra
        // plus dans les listes de jeu (filtrage cote query) mais ses donnees
        // non-jeu (infractions, audit, stats) sont conservees.
        let g = guild_id.to_string();
        let u = user.id.to_string();
        let grpc = ctx.data.read().await.get::<GrpcClientKey>().cloned();
        if let Some(grpc) = grpc {
            let req = proto_members::MemberLifecycleRequest {
                guild_id: g,
                user_id: u,
            };
            if let Err(e) = crate::grpc_call!(@unit &grpc, members, leave_member, req) {
                warn!(error = %e, "Echec callback leave membre");
            }
        }
    }

    async fn guild_member_update(
        &self,
        ctx: Context,
        old: Option<Member>,
        new_member: Option<Member>,
        event: serenity::model::event::GuildMemberUpdateEvent,
    ) {
        // Fin du filtrage d'adhesion Discord (rules screening) : `pending`
        // passe de true a false -> on attribue le(s) role(s) du reglement.
        let screening_done = old.as_ref().map(|m| m.pending).unwrap_or(false) && !event.pending;
        let (sg_guild, sg_user) = (event.guild_id, event.user.id);

        modules::audit::on_member_update(&ctx, old.clone(), new_member.clone(), event).await;

        if screening_done {
            modules::welcome::on_screening_complete(&ctx, sg_guild, sg_user).await;
        }
        if let Some(member) = new_member {
            modules::security::on_member_update(&ctx, &member).await;
            // Prefixe emoji staff : recompute le pseudo au changement de role.
            modules::progression::nickname::on_member_update(&ctx, &member).await;
        }
    }

    async fn channel_create(&self, ctx: Context, channel: GuildChannel) {
        modules::audit::on_channel_create(&ctx, &channel).await;
    }

    async fn channel_delete(
        &self,
        ctx: Context,
        channel: GuildChannel,
        messages: Option<Vec<Message>>,
    ) {
        modules::audit::on_channel_delete(&ctx, &channel, messages).await;
    }

    async fn guild_ban_addition(&self, ctx: Context, guild_id: GuildId, banned_user: User) {
        modules::audit::on_ban_add(&ctx, guild_id, &banned_user).await;
        modules::security::on_ban_add(&ctx, guild_id, &banned_user).await;
    }

    async fn guild_ban_removal(&self, ctx: Context, guild_id: GuildId, unbanned_user: User) {
        modules::audit::on_ban_remove(&ctx, guild_id, &unbanned_user).await;
        modules::security::on_ban_remove(&ctx, guild_id, &unbanned_user).await;
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        modules::audit::on_voice_state_update(&ctx, old.clone(), &new).await;
        modules::voice::on_voice_state_update(&ctx, &old, &new).await;
        modules::welcome::on_voice_state_update(&ctx, &old, &new).await;
        modules::progression::on_voice_state_update(&ctx, old, &new).await;

        // Presence publiee sur le site. En dernier : elle republie un
        // instantane complet, donc elle doit voir le cache une fois que les
        // autres modules ont fini d'agir dessus (creation ou suppression de
        // salon temporaire).
        if let Some(guild_id) = new.guild_id {
            modules::presence::on_voice_state_update(&ctx, guild_id).await;
        }
    }

    async fn guild_role_create(&self, ctx: Context, new: Role) {
        modules::audit::on_role_create(&ctx, &new).await;
    }

    async fn guild_role_delete(
        &self,
        ctx: Context,
        guild_id: GuildId,
        removed_role_id: RoleId,
        removed_role: Option<Role>,
    ) {
        modules::audit::on_role_delete(&ctx, guild_id, removed_role_id, removed_role).await;
    }

    async fn guild_role_update(&self, ctx: Context, old: Option<Role>, new: Role) {
        modules::audit::on_role_update(&ctx, old, &new).await;
    }

    /// Declenche quand le bot rejoint une nouvelle guild OU au re-sync au
    /// demarrage (is_new=Some(false) dans ce cas). On enregistre les
    /// slash commands + register cote API uniquement pour les vraies
    /// nouvelles guilds, sinon on duplique le travail deja fait dans `ready`.
    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: Option<bool>) {
        // Mono-serveur : le bot QUITTE toute autre guilde. L'installation ne
        // sert qu'un serveur, et y rester consommerait des evenements, de la
        // memoire et des quotas Discord pour des donnees que l'API refusera
        // de toute facon.
        //
        // Le controle vient avant `is_new` : une invitation acceptee pendant
        // une coupure du bot arrive au demarrage avec `is_new == None`.
        if !guilde_autorisee(guild.id) {
            tracing::warn!(
                guild_id = %guild.id,
                name = %guild.name,
                "mono-serveur : guilde non autorisee, le bot la quitte"
            );
            if let Err(e) = guild.id.leave(&ctx.http).await {
                tracing::error!(error = %e, guild_id = %guild.id, "echec du depart");
            }
            return;
        }

        if is_new != Some(true) {
            return;
        }
        info!(guild_id = %guild.id, name = %guild.name, "Bot ajoute a une nouvelle guild");

        // 1. Register cote API (heartbeat / dashboard)
        {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                let member_count = guild.member_count as i32;
                let owner_id = guild.owner_id.to_string();
                if let Err(e) = api
                    .register_guild(
                        &guild.id.to_string(),
                        &guild.name,
                        member_count,
                        Some(&owner_id),
                    )
                    .await
                {
                    tracing::warn!(error = %e, guild = %guild.name, "Erreur enregistrement guild");
                }
            }
        }

        // 2. Refresh slash commands pour cette guild
        crate::command_registry::refresh_guild_commands(&ctx, guild.id).await;
    }

    /// Declenche quand le bot est retire d'une guild (kick/ban/serveur
    /// supprime) OU lors d'une indisponibilite temporaire Discord (outage).
    /// On distingue les deux via `incomplete.unavailable` : si true, c'est un
    /// outage -> on ne supprime PAS (le serveur reviendra). Si false, le bot a
    /// reellement quitte -> on purge cote API pour que le selecteur web cesse
    /// d'afficher un serveur fantome.
    async fn guild_delete(
        &self,
        ctx: Context,
        incomplete: serenity::model::guild::UnavailableGuild,
        _full: Option<Guild>,
    ) {
        if incomplete.unavailable {
            // Outage Discord : indisponibilite temporaire, pas un retrait.
            return;
        }
        info!(guild_id = %incomplete.id, "Bot retire d'une guild");
        let api = ctx.data.read().await.get::<ApiClientKey>().cloned();
        if let Some(api) = api {
            if let Err(e) = api.delete_guild(&incomplete.id.to_string()).await {
                tracing::warn!(error = %e, guild_id = %incomplete.id, "Erreur suppression guild");
            }
        }
    }

    async fn guild_update(
        &self,
        ctx: Context,
        old: Option<Guild>,
        new_incomplete: serenity::model::guild::PartialGuild,
    ) {
        modules::audit::on_guild_update(&ctx, old, &new_incomplete).await;
    }

    async fn thread_create(&self, ctx: Context, thread: GuildChannel) {
        modules::audit::on_thread_create(&ctx, &thread).await;
    }

    async fn thread_delete(
        &self,
        ctx: Context,
        thread: serenity::model::channel::PartialGuildChannel,
        full_thread: Option<GuildChannel>,
    ) {
        modules::audit::on_thread_delete(&ctx, &thread, full_thread).await;
    }

    async fn invite_create(&self, ctx: Context, data: serenity::model::event::InviteCreateEvent) {
        modules::audit::on_invite_create(&ctx, &data).await;
    }

    async fn invite_delete(&self, ctx: Context, data: serenity::model::event::InviteDeleteEvent) {
        modules::audit::on_invite_delete(&ctx, &data).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                let name = command.data.name.as_str();

                // ── Telemetrie commande : invoked + success/error ──
                let api = {
                    let data = ctx.data.read().await;
                    data.get::<ApiClientKey>().cloned()
                };
                let full_cmd = format_full_command(&command.data);
                let module = command_module(name);
                let user_id = command.user.id.to_string();
                let user_name = command.user.name.clone();
                let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();

                // ANONYMAT : /confess ne doit JAMAIS lier l'auteur au module
                // confessions dans les logs (cf. revue securite). On coupe toute
                // telemetrie pour cette commande. (confess-admin reste loggue
                // pour la tracabilite des actions de moderation.)
                let log_telemetry = name != "confess";

                if let Some(ref api) = api {
                    if log_telemetry {
                        api.send_bot_log_with_details(
                            "info",
                            &format!("Commande invoquée : {full_cmd} (par @{user_name})"),
                            serde_json::json!({
                                "event_type": "command.invoked",
                                "command": full_cmd,
                                "module": module,
                                "user_id": user_id,
                                "user_name": user_name,
                                "guild_id": guild_id,
                            }),
                        );
                    }
                }

                // Log une-ligne des commandes admin/moderateur dans le salon
                // dedie et parametrable (opt-in via la config audit-bot).
                if !guild_id.is_empty() && is_admin_command(name) {
                    let reason = extract_reason(&command.data.options);
                    modules::audit::log_admin_command(
                        &ctx,
                        &guild_id,
                        &user_id,
                        &user_name,
                        &full_cmd,
                        reason.as_deref(),
                    )
                    .await;
                }

                let start = std::time::Instant::now();

                let dispatch = AssertUnwindSafe(async {
                    match name {
                        "purge" | "cleanup" => {
                            modules::cleanup::handle_command(&ctx, &command).await
                        }
                        "bump-statut" => modules::bump::handle_command(&ctx, &command).await,
                        "roles-panel" | "parrain" => {
                            modules::community::handle_command(&ctx, &command).await
                        }
                        "audit" => modules::audit::handle_command(&ctx, &command).await,
                        "logs-init" => modules::logs_setup::handle(&ctx, &command).await,
                        "level" | "stats" | "progression-resync" | "classement" => {
                            modules::progression::handle_command(&ctx, &command).await
                        }
                        "security" => modules::security::handle_command(&ctx, &command).await,
                        "automod" => modules::automod::handle_command(&ctx, &command).await,
                        "warn" | "unwarn" | "mute" | "unmute" | "ban" | "ban-sursis" | "unban"
                        | "history" | "note" | "call" | "signalement" | "context" | "appeal"
                        | "expirations" | "compare" | "modstats" | "evidence" | "review"
                        | "template" | "transcript" | "export" | "massmute" | "massban"
                        | "copilote" => modules::moderation::handle_command(&ctx, &command).await,
                        "ticket" | "ticket-admin" => {
                            modules::tickets::handle_command(&ctx, &command).await
                        }
                        "idee" => modules::ideas::handle_command(&ctx, &command).await,
                        "confess" | "confess-admin" => {
                            modules::confessions::handle_command(&ctx, &command).await
                        }
                        "backup" => modules::guild_backup::handle_command(&ctx, &command).await,
                        "apod" => modules::nasa_apod::handle_command(&ctx, &command).await,
                        _ => {}
                    }
                })
                .catch_unwind()
                .await;

                let elapsed_ms = start.elapsed().as_millis() as u64;

                if let Some(ref api) = api {
                    if log_telemetry {
                        match dispatch {
                            Ok(()) => api.send_bot_log_with_details(
                                "info",
                                &format!("Commande OK : {full_cmd} ({elapsed_ms} ms)"),
                                serde_json::json!({
                                    "event_type": "command.success",
                                    "command": full_cmd,
                                    "module": module,
                                    "user_id": user_id,
                                    "user_name": user_name,
                                    "guild_id": guild_id,
                                    "elapsed_ms": elapsed_ms,
                                }),
                            ),
                            Err(_) => api.send_bot_log_with_details(
                                "error",
                                &format!("Commande PANIC : {full_cmd}"),
                                serde_json::json!({
                                    "event_type": "command.error",
                                    "command": full_cmd,
                                    "module": module,
                                    "user_id": user_id,
                                    "user_name": user_name,
                                    "guild_id": guild_id,
                                    "elapsed_ms": elapsed_ms,
                                    "kind": "panic",
                                }),
                            ),
                        }
                    }
                }
            }
            Interaction::Component(component) => {
                let cid = component.data.custom_id.as_str();
                if modules::announcements::handles_component(cid) {
                    modules::announcements::on_component(&ctx, &component).await;
                } else if modules::confessions::handles_component(cid) {
                    modules::confessions::on_component(&ctx, &component).await;
                } else if modules::welcome::handles_component(cid) {
                    modules::welcome::on_component(&ctx, &component).await;
                } else if modules::community::handles_component(cid) {
                    modules::community::on_component(&ctx, &component).await;
                } else if modules::security::handles_component(cid) {
                    modules::security::on_component(&ctx, &component).await;
                } else if modules::automod::handles_component(cid) {
                    modules::automod::on_component(&ctx, &component).await;
                } else if modules::moderation::handles_component(cid) {
                    modules::moderation::on_component(&ctx, &component).await;
                } else if modules::voice::handles_component(cid) {
                    modules::voice::on_component(&ctx, &component).await;
                } else if modules::tickets::handles_component(cid) {
                    modules::tickets::on_component(&ctx, &component).await;
                } else if modules::ideas::handles_component(cid) {
                    modules::ideas::on_component(&ctx, &component).await;
                } else if modules::guild_backup::handles_component(cid) {
                    modules::guild_backup::on_component(&ctx, &component).await;
                }
            }
            Interaction::Modal(modal) => {
                let mcid = modal.data.custom_id.as_str();
                if modules::voice::handles_modal(mcid) {
                    modules::voice::on_modal(&ctx, &modal).await;
                } else if modules::tickets::handles_modal(mcid) {
                    modules::tickets::on_modal(&ctx, &modal).await;
                } else if modules::ideas::handles_modal(mcid) {
                    modules::ideas::on_modal(&ctx, &modal).await;
                } else if modules::confessions::handles_modal(mcid) {
                    modules::confessions::on_modal(&ctx, &modal).await;
                } else if modules::welcome::handles_modal(mcid) {
                    modules::welcome::on_modal(&ctx, &modal).await;
                }
            }
            Interaction::Autocomplete(autocomplete) => {
                let cmd_name = autocomplete.data.name.as_str();
                if modules::moderation::handles_autocomplete(cmd_name) {
                    modules::moderation::handle_autocomplete(&ctx, &autocomplete).await;
                }
            }
            _ => {}
        }
    }
}

/// La guilde est-elle celle servie par cette installation ?
///
/// `PUBLIC_GUILD_ID` absente = aucun verrou : une installation qui ne l'a pas
/// encore renseignee ne doit pas voir son bot quitter tous ses serveurs.
/// C'est le seul defaut sur : refuser par defaut ferait disparaitre le bot au
/// premier demarrage mal configure, et un depart ne se rattrape pas d'un clic.
fn guilde_autorisee(guild_id: serenity::model::id::GuildId) -> bool {
    let attendu = std::env::var("PUBLIC_GUILD_ID")
        .or_else(|_| std::env::var("GUILD_ID"))
        .unwrap_or_default();
    let attendu = attendu.trim();

    attendu.is_empty() || attendu == guild_id.to_string()
}
