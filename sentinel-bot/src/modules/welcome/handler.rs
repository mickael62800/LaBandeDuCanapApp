use std::sync::Arc;

use serenity::builder::{
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, EditChannel,
};
use serenity::model::guild::Member;
use serenity::model::id::{ChannelId, GuildId, RoleId};
use serenity::model::user::User;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::discord_helpers::is_module_enabled;
use crate::shared::grpc_client::{grpc_err_to_string, GrpcClientKey};
use crate::shared::heartbeat::ApiClientKey;
use sentinel_proto::age_gate::v1 as proto_age;

use super::api_client::WelcomeApiClient;
use super::template;

/// Reconstruit l'URL d'une banniere pour la rendre INDEPENDANTE du domaine.
///
/// Les bannieres du site sont stockees en base sous forme d'URL absolue
/// (`https://domaine-du-jour/imgs/x.jpg`), figee au domaine utilise le jour du
/// choix. Quand le site change d'adresse, Discord (qui va chercher l'image sur
/// Internet) tombe sur l'ancien domaine : la banniere disparait, sans erreur
/// nulle part. On repare a l'envoi : toute URL contenant `/imgs/` est
/// recomposee sur `WEB_FRONT_URL` (le domaine public courant, deja configure
/// pour l'OAuth). Les URLs externes (sans `/imgs/`) passent telles quelles.
///
/// Retourne `None` si `raw` est vide (pas d'image a poser).
fn resolve_banner_url(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    match raw.split_once("/imgs/") {
        Some((_, fichier)) => {
            let base = std::env::var("WEB_FRONT_URL").unwrap_or_default();
            let base = base.trim().trim_end_matches('/');
            if base.is_empty() {
                // Pas de domaine public connu : mieux vaut l'URL stockee que rien.
                Some(raw.to_string())
            } else {
                Some(format!("{base}/imgs/{fichier}"))
            }
        }
        None => Some(raw.to_string()),
    }
}

pub const RULES_ACCEPT_ID: &str = "sentinel_rules_accept";
/// custom_id du modal de saisie d'age (ouvert au clic sur "J'accepte" quand
/// la verification d'age est activee).
pub const AGE_MODAL_ID: &str = "sentinel_age_modal";
/// custom_id du champ de saisie de l'age dans le modal.
pub const AGE_INPUT_ID: &str = "age";

/// Decision de la verification d'age, prise server-side (seuil pass/ban +
/// duree). Le bot n'execute que l'action Discord correspondante. Reconstruite
/// depuis `AgeGateService::CheckAge` (gRPC).
#[derive(Debug)]
enum AgeCheckDecisionResponse {
    /// Age suffisant -> assignation du role membre.
    Grant,
    /// Age insuffisant -> ban temporaire jusqu'a `unban_at`.
    Ban {
        years: i32,
        /// Date de deban (RFC3339).
        unban_at: String,
        reason: String,
    },
}

/// Appele quand un nouveau membre rejoint.
/// Compte les HUMAINS (hors bots) via le cache de la guild ; repli sur le
/// compte approximatif Discord (qui inclut les bots) si le cache est vide.
async fn human_member_count(ctx: &Context, guild_id: GuildId) -> u64 {
    // Compte EXACT des humains via la liste HTTP des membres (List Guild
    // Members, pagine par lots de 1000). On exclut les bots (`user.bot`).
    //
    // On ne se fie plus a `member_count - bots_du_cache` : quand le cache des
    // membres n'est pas (encore) peuple, aucun bot n'y figure -> les bots
    // etaient comptes comme des humains. L'appel HTTP, lui, renvoie toujours le
    // flag bot correct (l'intent GUILD_MEMBERS est requis et actif : sans lui
    // le bot ne se connecterait pas).
    let mut humans: u64 = 0;
    let mut after: Option<serenity::model::id::UserId> = None;
    loop {
        match guild_id.members(&ctx.http, Some(1000), after).await {
            Ok(batch) if !batch.is_empty() => {
                humans += batch.iter().filter(|m| !m.user.bot).count() as u64;
                if batch.len() < 1000 {
                    break;
                }
                after = batch.last().map(|m| m.user.id);
            }
            Ok(_) => break,
            Err(e) => {
                warn!(error = %e, "Compteur : echec fetch membres HTTP, repli cache");
                // Repli sur le cache (total - bots vus). Peut inclure des bots
                // si le cache est vide, mais mieux que 0.
                if let Some(g) = ctx.cache.guild(guild_id) {
                    let total = g.member_count;
                    let bots = g.members.values().filter(|m| m.user.bot).count() as u64;
                    return total.saturating_sub(bots);
                }
                return humans;
            }
        }
    }
    humans
}

/// Renomme le salon compteur avec le nombre de membres. Independant des
/// messages welcome/leave : ne depend que de `counter_enabled`.
async fn update_counter(
    ctx: &Context,
    counter_enabled: bool,
    counter_channel_id: Option<&String>,
    counter_format: &str,
    member_count: u64,
) {
    if !counter_enabled {
        return;
    }
    let Some(ch_id) = counter_channel_id else {
        return;
    };
    let Ok(ch) = ch_id.parse::<u64>() else { return };
    let channel_id = ChannelId::new(ch);
    let name = counter_format.replace("{count}", &member_count.to_string());
    // N'ecrit QUE si le nom change reellement : Discord limite les renommages a
    // 2 / 10 min par salon. Un no-op gaspillerait ce quota (et le rafraichissement
    // periodique renommerait sans arret).
    if let Some(current) = channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild().map(|g| g.name))
    {
        if current == name {
            return;
        }
    }
    if let Err(e) = channel_id
        .edit(&ctx.http, EditChannel::new().name(&name))
        .await
    {
        warn!(error = %e, "Echec mise a jour compteur membres");
    }
}

/// Compte les HUMAINS (hors bots) actuellement connectes en vocal sur la
/// guild, via le cache des `voice_states`. Les bots musique/soundboard ne
/// sont pas comptes.
fn voice_member_count(ctx: &Context, guild_id: GuildId) -> u64 {
    let Some(g) = ctx.cache.guild(guild_id) else {
        return 0;
    };
    g.voice_states
        .values()
        .filter(|vs| vs.channel_id.is_some())
        .filter(|vs| {
            // Exclut les bots : si le membre est en cache et marque bot, on
            // l'ignore ; sinon on le compte (humain par defaut).
            !g.members
                .get(&vs.user_id)
                .map(|m| m.user.bot)
                .unwrap_or(false)
        })
        .count() as u64
}

/// Renomme le salon compteur vocal avec le nombre de connectes en vocal.
/// Independant du compteur de membres : ne depend que de
/// `voice_counter_enabled`.
async fn update_voice_counter(
    ctx: &Context,
    enabled: bool,
    channel_id: Option<&String>,
    format: &str,
    voice_count: u64,
) {
    if !enabled {
        return;
    }
    let Some(ch_id) = channel_id else { return };
    let Ok(ch) = ch_id.parse::<u64>() else { return };
    let channel_id = ChannelId::new(ch);
    let name = format.replace("{count}", &voice_count.to_string());
    // N'ecrit que si le nom change (cf. limite de rename Discord).
    if let Some(current) = channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild().map(|g| g.name))
    {
        if current == name {
            return;
        }
    }
    if let Err(e) = channel_id
        .edit(&ctx.http, EditChannel::new().name(&name))
        .await
    {
        warn!(error = %e, "Echec mise a jour compteur vocal");
    }
}

/// Recalcule et met a jour les compteurs (membres + vocal) pour une guild,
/// SANS dependre d'un event join/leave. Appele periodiquement et au demarrage :
/// sinon un compteur active alors que personne ne rejoint/part ne se met jamais
/// a jour. Ne renomme le salon que si le nombre a change (rate limit Discord).
pub async fn refresh_counters(ctx: &Context, guild_id: GuildId) {
    if !is_module_enabled(
        ctx,
        &guild_id.to_string(),
        crate::modules::welcome::MODULE_BOT_NAME,
    )
    .await
    {
        return;
    }

    let grpc = {
        let data = ctx.data.read().await;
        let grpc = data
            .get::<crate::shared::grpc_client::GrpcClientKey>()
            .map(Arc::clone);
        match grpc {
            Some(g) => g,
            _ => return,
        }
    };
    let api = WelcomeApiClient::new(grpc);
    let config = match api.get_config(&guild_id.to_string()).await {
        Ok(c) => c,
        Err(_) => return,
    };

    if config.counter_enabled {
        let count = human_member_count(ctx, guild_id).await;
        update_counter(
            ctx,
            true,
            config.counter_channel_id.as_ref(),
            &config.counter_format,
            count,
        )
        .await;
    }
    if config.voice_counter_enabled {
        let vcount = voice_member_count(ctx, guild_id);
        update_voice_counter(
            ctx,
            true,
            config.voice_counter_channel_id.as_ref(),
            &config.voice_counter_format,
            vcount,
        )
        .await;
    }
}

/// Appele a chaque changement d'etat vocal (join/leave/move). Met a jour le
/// salon compteur "En Vocal : N" si la fonctionnalite est activee.
pub async fn on_voice_state_update(
    ctx: &Context,
    old: &Option<serenity::model::voice::VoiceState>,
    new: &serenity::model::voice::VoiceState,
) {
    // guild_id provient de `new`, ou de `old` lors d'une deconnexion totale.
    let guild_id = match new
        .guild_id
        .or_else(|| old.as_ref().and_then(|o| o.guild_id))
    {
        Some(g) => g,
        None => return,
    };

    if !is_module_enabled(
        ctx,
        &guild_id.to_string(),
        crate::modules::welcome::MODULE_BOT_NAME,
    )
    .await
    {
        return;
    }

    let data = ctx.data.read().await;
    let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
        Some(g) => Arc::clone(g),
        None => return,
    };
    drop(data);

    let api = WelcomeApiClient::new(grpc);
    let config = match api.get_config(&guild_id.to_string()).await {
        Ok(c) => c,
        Err(_) => return,
    };

    if !config.voice_counter_enabled {
        return;
    }

    let voice_count = voice_member_count(ctx, guild_id);
    update_voice_counter(
        ctx,
        config.voice_counter_enabled,
        config.voice_counter_channel_id.as_ref(),
        &config.voice_counter_format,
        voice_count,
    )
    .await;
}

pub async fn on_member_add(ctx: &Context, new_member: &Member) {
    on_member_add_impl(ctx, new_member, false).await;
}

async fn on_member_add_impl(ctx: &Context, new_member: &Member, rules_accepted: bool) {
    // Les bots ajoutes au serveur ne doivent pas declencher le welcome / DM /
    // role de verification (ni recevoir le role "membre temporaire").
    if new_member.user.bot {
        return;
    }
    let ctx = ctx.clone();
    let new_member = new_member.clone();
    let guild_id = new_member.guild_id;
    let user_id = new_member.user.id;

    // Master switch : si le module est desactive, on saute tout
    // (welcome embed, DM, counter, etc.). Default true.
    if !is_module_enabled(
        &ctx,
        &guild_id.to_string(),
        crate::modules::welcome::MODULE_BOT_NAME,
    )
    .await
    {
        return;
    }

    // Depart eclair : on note l'arrivee AVANT tout appel reseau, pour que la
    // fenetre coure meme si la config ou l'envoi de la card echouent.
    super::ghost::remember_arrival(guild_id.get(), user_id.get());

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => Arc::clone(b),
        None => return,
    };
    let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
        Some(g) => Arc::clone(g),
        None => return,
    };
    drop(data);

    let api = WelcomeApiClient::new(grpc);
    let config = match api.get_config(&guild_id.to_string()).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "Echec chargement config welcome");
            return;
        }
    };

    // ── Verification d'age : role "Membre temporaire" a l'arrivee ──
    // Si active, le nouveau membre recoit le role d'attente (qui ne voit que
    // le reglement) ; il obtiendra le role Membre apres avoir saisi un age
    // suffisant via le formulaire du reglement.
    if config.age_check_enabled && !rules_accepted {
        if let Some(role) = config
            .unverified_role_id
            .as_deref()
            .and_then(|s| s.parse::<u64>().ok())
            .map(RoleId::new)
        {
            if let Err(e) = ctx
                .http
                .add_member_role(
                    guild_id,
                    user_id,
                    role,
                    Some("Verification d'age en attente"),
                )
                .await
            {
                warn!(error = %e, role = %role, "Echec attribution role Membre temporaire");
            }
        }
    }

    let guild_name = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name.clone())
        .unwrap_or_else(|_| "Serveur".into());

    let member_count = human_member_count(&ctx, guild_id).await;

    // ── Detecter si c'est un retour (membre deja connu) ──
    let is_rejoin = if rules_accepted {
        false
    } else {
        api.is_known_member(&guild_id.to_string(), &user_id.to_string())
            .await
    };

    // ── Message de bienvenue ──
    if config.welcome_enabled && (!config.rules_enabled || rules_accepted) {
        if let Some(ch_id) = &config.welcome_channel_id {
            if let Ok(ch) = ch_id.parse::<u64>() {
                let channel = ChannelId::new(ch);

                // Choisir le message : retour ou premiere fois
                let msg_template = if is_rejoin {
                    &config.rejoin_message
                } else {
                    &config.welcome_message
                };

                let text = template::render(
                    msg_template,
                    &user_id.to_string(),
                    &new_member.user.name,
                    &guild_name,
                    member_count,
                    None,
                );

                // Choix title/image/footer selon bienvenue vs retour.
                let (raw_title, raw_image, raw_footer, default_title) = if is_rejoin {
                    (
                        &config.rejoin_title,
                        &config.rejoin_image_url,
                        &config.rejoin_footer_text,
                        "Bon retour !",
                    )
                } else {
                    (
                        &config.welcome_title,
                        &config.welcome_image_url,
                        &config.welcome_footer_text,
                        "Bienvenue !",
                    )
                };
                let title = if raw_title.is_empty() {
                    default_title.to_string()
                } else {
                    raw_title.clone()
                };
                let color = template::parse_color(&config.welcome_embed_color);
                let footer_raw = if raw_footer.is_empty() {
                    format!("{} membres", member_count)
                } else {
                    raw_footer.replace("{count}", &member_count.to_string())
                };
                let mut embed = CreateEmbed::new()
                    .title(&title)
                    .description(&text)
                    .color(color)
                    .thumbnail(new_member.user.face())
                    .footer(CreateEmbedFooter::new(footer_raw));

                // L'image (banniere) est integree a l'embed : un seul message
                // est envoye. Discord l'affiche en grand sous le texte.
                if let Some(image_url) = resolve_banner_url(raw_image) {
                    embed = embed.image(image_url);
                }

                match channel
                    .send_message(&ctx.http, CreateMessage::new().embed(embed))
                    .await
                {
                    Err(e) => warn!(error = %e, "Echec envoi message bienvenue"),
                    Ok(sent) => {
                        // Retenu pour pouvoir retirer la card si le membre
                        // repart dans la foulee (cf. `ghost`).
                        super::ghost::attach_message(
                            guild_id.get(),
                            user_id.get(),
                            channel.get(),
                            sent.id.get(),
                        );
                        info!(
                            user = %new_member.user.name,
                            guild = %guild_name,
                            rejoin = is_rejoin,
                            "Message de {} envoye",
                            if is_rejoin { "retour" } else { "bienvenue" }
                        );
                    }
                }
            }
        }
    }

    // ── DM de bienvenue ──
    if config.welcome_dm_enabled && (!config.rules_enabled || rules_accepted) {
        let dm_text = template::render(
            &config.welcome_dm_message,
            &user_id.to_string(),
            &new_member.user.name,
            &guild_name,
            member_count,
            None,
        );

        if let Ok(dm_channel) = new_member.user.create_dm_channel(&ctx.http).await {
            if let Err(e) = dm_channel
                .send_message(&ctx.http, CreateMessage::new().content(&dm_text))
                .await
            {
                warn!(error = %e, user = %new_member.user.name, "Echec envoi DM bienvenue");
            }
        }
    }

    // ── Compteur de membres ──
    if !rules_accepted {
        update_counter(
            &ctx,
            config.counter_enabled,
            config.counter_channel_id.as_ref(),
            &config.counter_format,
            member_count,
        )
        .await;
    }

    if !config.rules_enabled || rules_accepted {
        base.publish_event(
            "atrium_welcome_requested",
            serde_json::json!({
                "guild_id": guild_id.to_string(),
                "user_id": user_id.to_string(),
            }),
        );
    }

    // ── Log ──
    if !rules_accepted {
        base.send_log(
            "info",
            &guild_id.to_string(),
            &format!("Nouveau membre : {} ({})", new_member.user.name, user_id),
        );
    }
}

async fn send_welcome_after_rules(
    ctx: &Context,
    guild_id: GuildId,
    user_id: serenity::all::UserId,
) {
    match guild_id.member(&ctx.http, user_id).await {
        Ok(member) => on_member_add_impl(ctx, &member, true).await,
        Err(e) => warn!(error = %e, %guild_id, %user_id, "Accueil apres reglement impossible"),
    }
}

/// Appele quand un membre quitte.
pub async fn on_member_remove(ctx: &Context, guild_id: GuildId, user: &User) {
    let ctx = ctx.clone();
    let user = user.clone();

    // Master switch : si le module est desactive, on saute le message de depart.
    if !is_module_enabled(
        &ctx,
        &guild_id.to_string(),
        crate::modules::welcome::MODULE_BOT_NAME,
    )
    .await
    {
        return;
    }

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => Arc::clone(b),
        None => return,
    };
    let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
        Some(g) => Arc::clone(g),
        None => return,
    };
    drop(data);

    let api = WelcomeApiClient::new(grpc);
    let config = match api.get_config(&guild_id.to_string()).await {
        Ok(c) => c,
        Err(_) => return,
    };

    // Compteur : INDEPENDANT du message de depart. On le met a jour AVANT
    // les early-returns ci-dessous (sinon un message de depart desactive
    // empechait la mise a jour du compteur au depart d'un membre).
    let member_count = human_member_count(&ctx, guild_id).await;
    update_counter(
        &ctx,
        config.counter_enabled,
        config.counter_channel_id.as_ref(),
        &config.counter_format,
        member_count,
    )
    .await;

    // Config guild du module (cles config_schema, distinctes de la config
    // welcome riche servie par gRPC). Lue une fois : fenetre fantome ci-dessous
    // et couleur de l'embed de depart plus bas.
    let guild_cfg = base
        .get_guild_config_for(
            &guild_id.to_string(),
            crate::modules::welcome::MODULE_BOT_NAME,
        )
        .await
        .unwrap_or_default();

    // ── Depart eclair ("fantome") ──
    // Un membre qui arrive et repart dans les `welcome_ghost_minutes` ne laisse
    // aucune trace dans le salon : on retire sa card de bienvenue et on
    // n'annonce pas son depart. Poster puis supprimer la card de depart aurait
    // produit un clignotement et une entree d'audit Discord pour rien.
    // 0 = comportement historique (on garde tout).
    let ghost_arrival = super::ghost::take(guild_id.get(), user.id.get());
    let ghost_minutes = crate::shared::api_client::BaseApiClient::config_u64(
        &guild_cfg,
        "welcome_ghost_minutes",
        30,
    );
    if ghost_minutes > 0 {
        if let Some(arrival) = ghost_arrival {
            if arrival.at.elapsed() <= std::time::Duration::from_secs(ghost_minutes * 60) {
                if let Some((ch, msg)) = arrival.message {
                    if let Err(e) = ctx
                        .http
                        .delete_message(
                            ChannelId::new(ch),
                            serenity::model::id::MessageId::new(msg),
                            Some("Depart du membre dans la fenetre de bienvenue"),
                        )
                        .await
                    {
                        warn!(error = %e, "Echec suppression card de bienvenue (depart eclair)");
                    }
                }
                info!(
                    user = %user.name,
                    minutes = ghost_minutes,
                    "Depart eclair : card de bienvenue retiree, card de depart supprimee"
                );
                return;
            }
        }
    }

    if !config.leave_enabled {
        return;
    }

    let ch_id = match &config.leave_channel_id {
        Some(c) => c,
        None => return,
    };

    let ch = match ch_id.parse::<u64>() {
        Ok(c) => ChannelId::new(c),
        Err(_) => return,
    };

    let guild_name = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name.clone())
        .unwrap_or_else(|_| "Serveur".into());

    let text = template::render(
        &config.leave_message,
        &user.id.to_string(),
        &user.name,
        &guild_name,
        member_count,
        None,
    );

    let leave_title = if config.leave_title.is_empty() {
        "Au revoir...".to_string()
    } else {
        config.leave_title.clone()
    };
    let leave_footer = if config.leave_footer_text.is_empty() {
        format!("{} membres", member_count)
    } else {
        config
            .leave_footer_text
            .replace("{count}", &member_count.to_string())
    };
    // Couleur de l'embed de depart : reglable par serveur (defaut historique
    // e74c3c). Lue via la config guild welcome-bot, parsee comme welcome_embed_color.
    let leave_color = template::parse_color(&crate::shared::api_client::BaseApiClient::config_or(
        &guild_cfg,
        "leave_embed_color",
        "e74c3c",
    ));
    let mut embed = CreateEmbed::new()
        .title(&leave_title)
        .description(&text)
        .color(leave_color)
        .footer(CreateEmbedFooter::new(leave_footer));

    // L'image est integree a l'embed : un seul message, image en grand sous le texte.
    if let Some(image_url) = resolve_banner_url(&config.leave_image_url) {
        embed = embed.image(image_url);
    }

    if let Err(e) = ch
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        warn!(error = %e, "Echec envoi message depart");
    }
    // (Le compteur a deja ete mis a jour plus haut, avant le return.)
}

/// Appele pour les interactions de composants (bouton reglement).
pub async fn on_component(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    if let Some(guild_id) = component.guild_id {
        if !is_module_enabled(
            ctx,
            &guild_id.to_string(),
            crate::modules::welcome::MODULE_BOT_NAME,
        )
        .await
        {
            return;
        }
    }
    if component.data.custom_id == RULES_ACCEPT_ID {
        handle_rules_accept(ctx, component).await;
    }
}

/// Gere le clic sur le bouton "J'accepte les regles".
/// Poste (ou republie) le panneau de reglement avec le bouton d'acceptation
/// dans le salon configure. Declenche par l'event `welcome_rules_publish`
/// (bouton "Publier le reglement" du dashboard).
pub async fn publish_rules_panel(ctx: &Context, guild_id: GuildId) -> Result<(), String> {
    use serenity::all::ButtonStyle;
    use serenity::builder::{CreateActionRow, CreateButton};

    let (base, grpc) = {
        let data = ctx.data.read().await;
        let base = data
            .get::<ApiClientKey>()
            .map(Arc::clone)
            .ok_or("client API absent")?;
        let grpc = data
            .get::<crate::shared::grpc_client::GrpcClientKey>()
            .map(Arc::clone)
            .ok_or("client gRPC absent")?;
        (base, grpc)
    };

    let api = WelcomeApiClient::new(grpc);
    let config = api
        .get_config(&guild_id.to_string())
        .await
        .map_err(|e| format!("lecture config welcome: {e}"))?;

    if !config.rules_enabled {
        return Err("la validation du reglement est desactivee".into());
    }
    let channel_id = config
        .rules_channel_id
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(ChannelId::new)
        .ok_or("aucun salon de reglement configure")?;

    let label = {
        let l = config.rules_button_label.trim();
        if l.is_empty() {
            "J'accepte les règles".to_string()
        } else {
            l.to_string()
        }
    };

    // Couleur du panneau de reglement : reglable par serveur (defaut 5865f2).
    let rules_color = {
        let cfg = base
            .get_guild_config_for(
                &guild_id.to_string(),
                crate::modules::welcome::MODULE_BOT_NAME,
            )
            .await
            .unwrap_or_default();
        template::parse_color(&crate::shared::api_client::BaseApiClient::config_or(
            &cfg,
            "rules_embed_color",
            "5865f2",
        ))
    };
    let embed = CreateEmbed::new()
        .title("📜 Règlement")
        .description(&config.rules_message)
        .color(rules_color);
    let button = CreateButton::new(RULES_ACCEPT_ID)
        .label(label)
        .style(ButtonStyle::Success);
    let row = CreateActionRow::Buttons(vec![button]);

    channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed).components(vec![row]),
        )
        .await
        .map_err(|e| format!("envoi du message: {e}"))?;

    info!(guild = %guild_id, channel = %channel_id, "Panneau de reglement publie");
    Ok(())
}

/// Attribue les role(s) (CSV d'IDs) a un membre. Fonction pure : la config
/// est deja lue par l'appelant. Retourne le nombre de roles poses.
async fn assign_roles_csv(
    ctx: &Context,
    guild_id: GuildId,
    user_id: serenity::model::id::UserId,
    role_csv: Option<&str>,
) -> usize {
    // CSV d'IDs : un ancien reglage a role unique reste un CSV a 1 element.
    let role_ids: Vec<RoleId> = role_csv
        .unwrap_or("")
        .split(',')
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .map(RoleId::new)
        .collect();

    // GARDE ANTI-ESCALADE : on n'auto-attribue JAMAIS un role privilegie /
    // managed / introuvable. Un rules_role_id/unverified_role_id mal configure
    // (ou pointant sur un role admin sous le bot) ne doit pas donner ce role a
    // tout arrivant qui accepte les regles / complete le screening.
    let dangerous = serenity::all::Permissions::ADMINISTRATOR
        | serenity::all::Permissions::MANAGE_GUILD
        | serenity::all::Permissions::MANAGE_ROLES
        | serenity::all::Permissions::MANAGE_CHANNELS
        | serenity::all::Permissions::MANAGE_WEBHOOKS
        | serenity::all::Permissions::BAN_MEMBERS
        | serenity::all::Permissions::KICK_MEMBERS
        | serenity::all::Permissions::MODERATE_MEMBERS
        | serenity::all::Permissions::MANAGE_MESSAGES
        | serenity::all::Permissions::MENTION_EVERYONE
        | serenity::all::Permissions::MANAGE_NICKNAMES
        | serenity::all::Permissions::MANAGE_THREADS
        | serenity::all::Permissions::MANAGE_EVENTS;

    let mut assigned = 0usize;
    for role_id in &role_ids {
        let safe = ctx
            .cache
            .guild(guild_id)
            .map(|g| match g.roles.get(role_id) {
                Some(r) => !r.managed && (r.permissions & dangerous).is_empty(),
                None => false,
            })
            .unwrap_or(false);
        if !safe {
            warn!(role = %role_id, "Refus auto-attribution d'un role privilegie/introuvable (welcome)");
            continue;
        }
        match ctx
            .http
            .add_member_role(guild_id, user_id, *role_id, Some("Reglement accepte"))
            .await
        {
            Ok(_) => assigned += 1,
            Err(e) => warn!(error = %e, role = %role_id, "Echec assignation role reglement"),
        }
    }
    assigned
}

/// Fin du filtrage d'adhesion Discord (membership screening) : `pending`
/// passe de true a false. On attribue alors le(s) role(s) du reglement —
/// SAUF si la verification d'age est active (le role Membre ne doit etre
/// donne qu'apres saisie d'un age suffisant, via le bouton + formulaire).
pub async fn on_screening_complete(
    ctx: &Context,
    guild_id: GuildId,
    user_id: serenity::model::id::UserId,
) {
    let config = match load_welcome_config(ctx, guild_id).await {
        Some(c) => c,
        None => return,
    };
    if !config.rules_enabled {
        return;
    }
    // Verif d'age active : on NE donne PAS le role ici (le membre doit passer
    // par le formulaire d'age). Il garde son role "Membre temporaire".
    if config.age_check_enabled {
        return;
    }
    let n = assign_roles_csv(ctx, guild_id, user_id, config.rules_role_id.as_deref()).await;
    match Ok::<usize, String>(n) {
        Ok(n) if n > 0 => {
            info!(user = %user_id, guild = %guild_id, roles = n, "Roles reglement attribues (filtrage Discord)");
            send_welcome_after_rules(ctx, guild_id, user_id).await;
        }
        Ok(_) => {}
        // Desactive / non configure : silencieux (cas normal sur la plupart
        // des serveurs). Les vraies erreurs d'assignation sont deja loggees.
        Err(_) => {}
    }
}

async fn handle_rules_accept(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };

    // UNE seule lecture de config (eviter le timeout 3s de l'interaction :
    // un Modal doit etre la 1re reponse, donc on doit decider avant de
    // repondre, mais sans enchainer plusieurs appels gRPC).
    let config = match load_welcome_config(ctx, guild_id).await {
        Some(c) => c,
        None => return,
    };
    if !config.rules_enabled {
        return;
    }

    // Verification d'age activee -> ouvrir le formulaire (au lieu d'attribuer
    // directement le role). L'attribution (ou le ban) se fait au submit.
    if config.age_check_enabled {
        let q = config.age_modal_question.trim();
        let q = if q.is_empty() {
            "Quel age as-tu ? (en chiffres)".to_string()
        } else {
            q.to_string()
        };
        open_age_modal(ctx, component, &q).await;
        return;
    }

    // Flux classique : attribuer le(s) role(s) depuis la config deja lue.
    let assigned = assign_roles_csv(
        ctx,
        guild_id,
        component.user.id,
        config.rules_role_id.as_deref(),
    )
    .await;

    let content = if assigned == 0 {
        "Erreur lors de l'assignation des roles (aucun role configure ?)."
    } else {
        "Reglement accepte ! Bienvenue sur le serveur."
    };
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec reponse acceptation reglement");
    }

    if assigned > 0 {
        send_welcome_after_rules(ctx, guild_id, component.user.id).await;
    }

    info!(user = %component.user.name, guild = %guild_id, assigned, "Reglement accepte");
}

// ─────────────────────────────────────────────────────────────────────────
// Verification d'age au reglement.
// ─────────────────────────────────────────────────────────────────────────

/// Lit la config welcome d'une guild (helper factorise).
async fn load_welcome_config(
    ctx: &Context,
    guild_id: GuildId,
) -> Option<super::api_client::WelcomeConfig> {
    let grpc = {
        let data = ctx.data.read().await;
        data.get::<crate::shared::grpc_client::GrpcClientKey>()
            .map(Arc::clone)?
    };
    WelcomeApiClient::new(grpc)
        .get_config(&guild_id.to_string())
        .await
        .ok()
}

/// Indique si la verification d'age est active (et le reglement actif) sur
/// cette guild. Utilise par d'autres modules (ex. community auto-roles) pour
/// suspendre l'attribution automatique de roles tant que le membre n'a pas
/// passe la verification.
pub async fn age_check_active(ctx: &Context, guild_id: GuildId) -> bool {
    match load_welcome_config(ctx, guild_id).await {
        Some(c) => c.rules_enabled && c.age_check_enabled,
        None => false,
    }
}

/// Ouvre le formulaire de saisie d'age.
async fn open_age_modal(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
    question: &str,
) {
    use serenity::builder::{CreateActionRow, CreateInputText, CreateModal};
    use serenity::model::application::InputTextStyle;

    let label: String = question.chars().take(45).collect();
    let modal = CreateModal::new(AGE_MODAL_ID, "Verification").components(vec![
        CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Short, label, AGE_INPUT_ID)
                .min_length(1)
                .max_length(3)
                .required(true),
        ),
    ]);
    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await
    {
        warn!(error = %e, "Echec ouverture modale age");
    }
}

fn extract_modal_input(
    modal: &serenity::model::application::ModalInteraction,
    field_id: &str,
) -> Option<String> {
    for row in &modal.data.components {
        for c in &row.components {
            if let serenity::all::ActionRowComponent::InputText(it) = c {
                if it.custom_id == field_id {
                    return it.value.clone();
                }
            }
        }
    }
    None
}

/// Submit du formulaire d'age : age suffisant -> role Membre ; sinon ban
/// temporaire jusqu'aux `age_minimum` ans.
pub async fn handle_age_modal(
    ctx: &Context,
    modal: &serenity::model::application::ModalInteraction,
) {
    let guild_id = match modal.guild_id {
        Some(g) => g,
        None => return,
    };
    let user_id = modal.user.id;

    // ACK immediat : les operations suivantes (lecture config gRPC,
    // ajout/retrait de roles, ban) peuvent depasser les 3s d'une interaction.
    // On differe, puis on repond via followup (reply_modal).
    if let Err(e) = modal
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec defer modale age");
        return;
    }

    let config = match load_welcome_config(ctx, guild_id).await {
        Some(c) => c,
        None => return,
    };

    // Bornes de saisie (validation d'entree) + salon de log : restent cote bot.
    // La DECISION age-check (seuil pass/ban + duree du ban) est server-side :
    // `age_ban_days_per_year` n'est donc plus lu ici.
    let (age_min, age_max, age_ban_log_channel) = {
        use crate::shared::api_client::BaseApiClient;
        let cfg = {
            let data = ctx.data.read().await;
            match data.get::<ApiClientKey>() {
                Some(base) => base
                    .get_guild_config_for(
                        &guild_id.to_string(),
                        crate::modules::welcome::MODULE_BOT_NAME,
                    )
                    .await
                    .unwrap_or_default(),
                None => std::collections::HashMap::new(),
            }
        };
        let read_min = BaseApiClient::config_u64(&cfg, "age_min", 5) as i32;
        let read_max = BaseApiClient::config_u64(&cfg, "age_max", 120) as i32;
        // Gardes : age_min >= 0 et age_min <= age_max, sinon retour aux defauts.
        let (age_min, age_max) = if read_min >= 0 && read_min <= read_max {
            (read_min, read_max)
        } else {
            (5, 120)
        };
        // Salon de log des bans de verification d'age (optionnel, configurable).
        let log_channel = BaseApiClient::config_or(&cfg, "age_ban_log_channel_id", "")
            .parse::<u64>()
            .ok()
            .filter(|id| *id > 0)
            .map(ChannelId::new);
        (age_min, age_max, log_channel)
    };

    // Parse de l'age saisi.
    let raw = extract_modal_input(modal, AGE_INPUT_ID).unwrap_or_default();
    let age: Option<i32> = raw
        .trim()
        .parse::<i32>()
        .ok()
        .filter(|a| (age_min..=age_max).contains(a));
    let age = match age {
        Some(a) => a,
        None => {
            reply_modal(
                ctx,
                modal,
                "Age invalide. Recommence en saisissant un nombre.",
            )
            .await;
            return;
        }
    };

    // DECISION age-check server-side : le bot delegue la regle metier (seuil
    // pass/ban + duree du ban) a l'API et n'execute que l'action Discord.
    let decision = {
        let grpc = {
            let data = ctx.data.read().await;
            data.get::<GrpcClientKey>().map(Arc::clone)
        };
        let grpc = match grpc {
            Some(g) => g,
            None => {
                warn!(guild = %guild_id, "API indisponible pour la verification d'age");
                reply_modal(
                    ctx,
                    modal,
                    "Verification d'age indisponible, reessaie plus tard.",
                )
                .await;
                return;
            }
        };
        let req = proto_age::CheckAgeRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            declared_age: age,
        };
        match crate::grpc_call!(&grpc, age_gate, check_age, req) {
            Ok(d) if d.grant => AgeCheckDecisionResponse::Grant,
            Ok(d) => AgeCheckDecisionResponse::Ban {
                years: d.years,
                unban_at: d.unban_at,
                reason: d.reason,
            },
            Err(e) => {
                warn!(error = %e, guild = %guild_id, "Echec decision age-check server-side");
                reply_modal(
                    ctx,
                    modal,
                    "Verification d'age indisponible, reessaie plus tard.",
                )
                .await;
                return;
            }
        }
    };

    // Age insuffisant -> ban temporaire (duree decidee server-side).
    if let AgeCheckDecisionResponse::Ban {
        years,
        unban_at,
        reason,
    } = &decision
    {
        let years = *years;
        let unban_at = match chrono::DateTime::parse_from_rfc3339(unban_at) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(e) => {
                warn!(error = %e, "unban_at invalide dans la decision age-check");
                return;
            }
        };
        let message = config
            .age_ban_message
            .replace("{min}", &config.age_minimum.to_string())
            .replace("{annees}", &years.to_string());

        // On repond AVANT le ban (l'interaction serait perdue sinon).
        reply_modal(ctx, modal, &message).await;

        if let Err(e) = guild_id
            .ban_with_reason(&ctx.http, user_id, 0, reason)
            .await
        {
            warn!(error = %e, user = %user_id, "Echec ban verification d'age");
            return;
        }

        // Enregistre le ban (source de verite du deban automatique par le worker).
        if let Some(grpc) = {
            let data = ctx.data.read().await;
            data.get::<GrpcClientKey>().map(Arc::clone)
        } {
            let req = proto_age::RecordAgeBanRequest {
                guild_id: guild_id.to_string(),
                user_id: user_id.to_string(),
                declared_age: age,
                unban_at: unban_at.to_rfc3339(),
            };
            if let Err(e) = crate::grpc_call!(@unit &grpc, age_gate, record_age_ban, req) {
                warn!(error = %e, "Echec enregistrement age-ban (deban auto compromis)");
            }
        }

        // Log staff : card informant qu'un membre a ete banni par la verification
        // d'age (salon configurable `age_ban_log_channel_id`, sinon rien). Best-effort.
        if let Some(channel) = age_ban_log_channel {
            let total_days = (unban_at - chrono::Utc::now()).num_days().max(0);
            let embed = CreateEmbed::new().color(0xE74C3Cu32).description(format!(
                "\u{1f51e} **Ban verification d'age** \u{2014} <@{uid}> (`{uid}`)\n\
                 Age declare : **{age} ans** (minimum {min}) \u{00b7} duree : **{years} an(s)** \
                 (~{days} j) \u{00b7} deban auto <t:{ts}:R>",
                uid = user_id.get(),
                age = age,
                min = config.age_minimum,
                years = years,
                days = total_days,
                ts = unban_at.timestamp(),
            ));
            if let Err(e) = channel
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await
            {
                warn!(error = %e, "Echec log ban verification d'age");
            }
        }

        info!(user = %modal.user.name, guild = %guild_id, age, years, "Age insuffisant -> ban temporaire");
        return;
    }

    // Age suffisant -> donne le role Membre PUIS retire le role temporaire.
    // Ordre important : on ajoute Membre avant de retirer le temporaire pour
    // qu'il n'y ait jamais d'instant ou le membre n'a aucun role d'acces.
    {
        let assigned =
            assign_roles_csv(ctx, guild_id, user_id, config.rules_role_id.as_deref()).await;

        if let Some(role) = config
            .unverified_role_id
            .as_deref()
            .and_then(|s| s.parse::<u64>().ok())
            .map(RoleId::new)
        {
            if let Err(e) = ctx
                .http
                .remove_member_role(guild_id, user_id, role, Some("Age verifie"))
                .await
            {
                warn!(error = %e, role = %role, "Echec retrait role Membre temporaire");
            }
        }

        if assigned > 0 {
            reply_modal(ctx, modal, "Bienvenue sur le serveur ! Acces accorde.").await;
            send_welcome_after_rules(ctx, guild_id, user_id).await;
            info!(user = %modal.user.name, guild = %guild_id, age, "Age verifie -> Membre");
        } else {
            reply_modal(
                ctx,
                modal,
                "Acces accorde, mais aucun role Membre n'est configure.",
            )
            .await;
            warn!(guild = %guild_id, "Age verifie mais aucun role Membre (rules_role_id) configure");
        }
    }
}

async fn reply_modal(
    ctx: &Context,
    modal: &serenity::model::application::ModalInteraction,
    content: &str,
) {
    // L'interaction a deja ete differee (Defer) au debut de handle_age_modal :
    // on repond donc via un followup, pas un create_response.
    let followup = serenity::builder::CreateInteractionResponseFollowup::new()
        .content(content)
        .ephemeral(true);
    if let Err(e) = modal.create_followup(&ctx.http, followup).await {
        warn!(error = %e, "Echec reponse modale age");
    }
}

/// Debannit un membre dont le ban d'age est arrive a echeance (event
/// `age_ban_lift` emis par le worker).
pub async fn lift_age_ban(ctx: &Context, guild_id: GuildId, user_id: u64) {
    let uid = serenity::model::id::UserId::new(user_id);
    match guild_id.unban(&ctx.http, uid).await {
        Ok(_) => info!(guild = %guild_id, user = user_id, "Ban d'age leve (deban)"),
        Err(e) => warn!(error = %e, guild = %guild_id, user = user_id, "Echec deban age"),
    }
}
