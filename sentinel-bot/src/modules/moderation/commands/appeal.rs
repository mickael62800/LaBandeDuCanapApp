use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use serenity::model::id::UserId;
use tracing::{info, warn};

use crate::shared::discord_helpers::reply_ephemeral;
use crate::shared::grpc_client::{grpc_err_to_string, GrpcClientKey};
use crate::shared::heartbeat::ApiClientKey;
use platform_proto::sentinel::moderation::v1 as proto_mod;
use platform_proto::sentinel::tickets::v1 as proto_tickets;

pub const APPEAL_PREFIX: &str = "sentinel_mod_appeal_";
/// Bouton modo « Voter pour annuler » : `mod_appeal_votecancel_{action_id}`.
pub const APPEAL_VOTE_PREFIX: &str = "mod_appeal_votecancel_";
/// Bouton admin « Valider l'annulation » : `mod_appeal_validate_{action_id}`.
pub const APPEAL_VALIDATE_PREFIX: &str = "mod_appeal_validate_";
/// Bouton modo « Fermer + bannir » (etape 1) : `mod_appeal_banclose_{user_id}`.
pub const APPEAL_BANCLOSE_PREFIX: &str = "mod_appeal_banclose_";
/// Confirmation du ban (etape 2) : `mod_appeal_banconfirm_{user_id}`.
pub const APPEAL_BANCONFIRM_PREFIX: &str = "mod_appeal_banconfirm_";
/// Bouton modo « Fermer le salon » (supprime le salon d'appel).
pub const APPEAL_CLOSE_ID: &str = "mod_appeal_close";

/// Votes d'annulation en cours : `action_id -> set des user_id moderateurs`.
/// In-process (les votes repartent a zero au redemarrage — acceptable, un appel
/// est de courte duree).
fn cancel_votes() -> &'static Mutex<HashMap<String, std::collections::HashSet<String>>> {
    static MAP: OnceLock<Mutex<HashMap<String, std::collections::HashSet<String>>>> =
        OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

/// MOD #9 — fenetre anti-spam de `/appeal` par (guild, user).
const APPEAL_COOLDOWN: Duration = Duration::from_secs(5 * 60);

/// Garde in-process (guild_id, user_id) -> dernier appel accepte.
fn appeal_cooldowns() -> &'static Mutex<HashMap<(String, String), Instant>> {
    static MAP: OnceLock<Mutex<HashMap<(String, String), Instant>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Tente de démarrer un appel. Renvoie `true` si autorisé, `false` si en cooldown.
/// Purge au passage les entrees expirees pour borner la memoire.
fn try_start_appeal(guild_id: &str, user_id: &str) -> bool {
    let mut map = appeal_cooldowns().lock().unwrap();
    let now = Instant::now();
    map.retain(|_, last| now.duration_since(*last) < APPEAL_COOLDOWN);

    let key = (guild_id.to_string(), user_id.to_string());
    // `entry` plutot que `contains_key` + `insert` : une seule recherche de
    // hash, et l'insertion ne peut pas diverger du test qui la precede.
    match map.entry(key) {
        std::collections::hash_map::Entry::Occupied(_) => false,
        std::collections::hash_map::Entry::Vacant(slot) => {
            slot.insert(now);
            true
        }
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("appeal")
        .description("Contester une sanction recue (cree un ticket automatiquement)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let user_id = command.user.id.to_string();

    // MOD #9 (b) — anti-spam : cooldown in-process par (guild, user) atomique.
    if !try_start_appeal(&guild_id.to_string(), &user_id) {
        reply_ephemeral(
            ctx,
            command,
            "Vous avez deja soumis un appel recemment. Patientez quelques minutes avant de reessayer.",
        )
        .await;
        return;
    }

    let grpc = match ctx.data.read().await.get::<GrpcClientKey>().cloned() {
        Some(g) => g,
        None => {
            reply_ephemeral(ctx, command, "Erreur interne.").await;
            return;
        }
    };

    // MOD #9 (a) — verifier que l'appelant a bien une sanction a contester.
    // En cas d'erreur reseau on reste permissif (on n'empeche pas un appel
    // legitime), mais une absence confirmee de sanction stoppe la creation.
    let hist_req = proto_mod::GetHistoryRequest {
        guild_id: guild_id.to_string(),
        user_id: user_id.clone(),
    };
    match crate::grpc_call!(&grpc, moderation, get_history, hist_req) {
        Ok(history) => {
            if history.actions.is_empty() {
                reply_ephemeral(
                    ctx,
                    command,
                    "Aucune sanction a contester n'a ete trouvee a votre encontre sur ce serveur.",
                )
                .await;
                return;
            }
        }
        Err(e) => {
            warn!(error = %e, "Verification sanction /appeal echouee, on laisse passer");
        }
    }

    // Ticket dashboard (best-effort, pour le suivi cote web).
    let ticket_req = proto_tickets::CreateTicketRequest {
        title: format!("Appel de sanction — {}", command.user.name),
        priority: "medium".to_string(),
        author_id: command.user.id.to_string(),
        author_name: command.user.name.clone(),
        server: guild_id.to_string(),
        category: "appel_sanction".to_string(),
        ticket_type: "appel_sanction".to_string(),
        channel_id: None,
        guild_id: Some(guild_id.to_string()),
    };
    if let Err(e) = crate::grpc_call!(@unit &grpc, tickets, create_ticket, ticket_req) {
        warn!(error = %e, "Ticket appel (dashboard) non cree — on continue");
    }

    finalize_appeal(
        ctx,
        &guild_id.to_string(),
        command.user.id.get(),
        &command.user.name,
        None,
        |content| {
            let ctx = ctx.clone();
            async move {
                reply_ephemeral(&ctx, command, &content).await;
            }
        },
    )
    .await;
    info!(user = %command.user.name, "Appel de sanction traite via /appeal");
}

/// Texte par defaut du « mode d'emploi » de l'appel (si config vide). Editable
/// via le dashboard (cle `appeal_guidelines`).
pub const DEFAULT_GUIDELINES: &str = "\
**📎 Ce qu'on attend de toi**\n\
• Des **preuves** concrètes : captures d'écran, liens de messages, dates, contexte.\n\
• Reste **factuel** et **respectueux** : pas d'insultes ni d'accusations gratuites.\n\
• Un appel n'est pas une 2ᵉ dispute — apporte des éléments **nouveaux ou vérifiables**.\n\n\
**⚖️ Tes droits**\n\
• Être **écouté** et obtenir un **réexamen** de ta sanction.\n\
• Si le problème concerne **un modérateur en particulier**, tu peux **demander qu'il ne participe pas** à ce salon (conflit d'intérêt) : indique-le clairement ici, un autre membre du staff prendra le relais.\n\
• La décision d'annuler une sanction n'est **jamais** prise par un seul modo : plusieurs votent, puis un **admin valide**.\n\n\
**🚫 Tes devoirs**\n\
• **Honnêteté** : mentir ou falsifier des preuves aggrave ta situation.\n\
• Un appel **abusif** (spam, insultes, mauvaise foi) peut être **refusé** et mener à un **bannissement**.";

/// Construit la carte « mode d'emploi » de l'appel (partagee appel + sursis).
/// Le texte des regles est parametrable (`appeal_guidelines`), `context` ajoute
/// une note en tete (ex. l'echeance d'un sursis).
pub async fn guidelines_embed(
    ctx: &Context,
    guild_id: &str,
    appellant_id: u64,
    action_id: Option<&str>,
    context: Option<&str>,
) -> serenity::builder::CreateEmbed {
    let guidelines = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(api) => api
                .get_guild_config_for(guild_id, crate::modules::moderation::MODULE_BOT_NAME)
                .await
                .ok()
                .and_then(|cfg| {
                    cfg.get("appeal_guidelines")
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or_else(|| DEFAULT_GUIDELINES.to_string()),
            None => DEFAULT_GUIDELINES.to_string(),
        }
    };

    let mut desc = String::new();
    if let Some(c) = context {
        desc.push_str(c);
        desc.push_str("\n\n");
    }
    desc.push_str(&format!(
        "<@{appellant_id}> conteste une sanction et demande un réexamen. Ce salon est \
         **privé** : seuls toi et l'équipe de modération le voyez. Expose calmement ta \
         version — l'objectif est de vérifier si la sanction est justifiée.\n\n"
    ));
    if let Some(a) = action_id {
        desc.push_str(&format!(
            "**Référence de l'action :** `{}`\n\n",
            &a[..16.min(a.len())]
        ));
    }
    desc.push_str(&guidelines);

    crate::shared::embeds::info_embed("📨 Appel de sanction — mode d'emploi")
        .description(desc)
        .timestamp(serenity::model::Timestamp::now())
}

/// Cree le salon d'appel (si categorie configuree) + notifie ; puis renvoie le
/// message a afficher a l'appelant via `reply`.
async fn finalize_appeal<F, Fut>(
    ctx: &Context,
    guild_id: &str,
    appellant_id: u64,
    appellant_name: &str,
    action_id: Option<&str>,
    reply: F,
) where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let intro = guidelines_embed(ctx, guild_id, appellant_id, action_id, None).await;

    // Boutons modo. Annulation d'une sanction = VOTE de modos + validation ADMIN.
    // Ban+fermeture = confirmation en 2 clics. Fermer = clore sans sanction.
    use serenity::all::{ButtonStyle, CreateButton};
    let mut buttons = Vec::new();
    if let Some(aid) = action_id {
        buttons.push(
            CreateButton::new(format!("{APPEAL_VOTE_PREFIX}{aid}"))
                .label("Voter : annuler")
                .emoji('🗳')
                .style(ButtonStyle::Secondary),
        );
        buttons.push(
            CreateButton::new(format!("{APPEAL_VALIDATE_PREFIX}{aid}"))
                .label("Valider l'annulation (admin)")
                .emoji('✅')
                .style(ButtonStyle::Success),
        );
    }
    buttons.push(
        CreateButton::new(format!("{APPEAL_BANCLOSE_PREFIX}{appellant_id}"))
            .label("Fermer + bannir")
            .emoji('🔨')
            .style(ButtonStyle::Danger),
    );
    buttons.push(
        CreateButton::new(APPEAL_CLOSE_ID)
            .label("Fermer le salon")
            .emoji('🔒')
            .style(ButtonStyle::Secondary),
    );

    // 1) Salon dedie sous la categorie (si configuree).
    if let Some(channel) = crate::modules::moderation::create_appeal_channel(
        ctx,
        guild_id,
        appellant_id,
        appellant_name,
        intro,
        buttons,
    )
    .await
    {
        reply(format!(
            "✅ Ton appel est ouvert : <#{channel}>. Un modérateur va l'examiner."
        ))
        .await;
        return;
    }

    // Pas de salon cree = categorie d'appel non configuree. On NE retombe plus
    // sur une notification dans un salon (systeme retire) : l'appel passe
    // exclusivement par un salon dedie cree sous la categorie.
    let _ = appellant_id;
    reply(
        "⚠️ Le système d'appel n'est pas configuré (catégorie d'appel manquante). \
         Contacte un modérateur — dashboard → Modération → « Catégorie des salons d'appel »."
            .to_string(),
    )
    .await;
}

pub async fn handle_appeal_button(ctx: &Context, component: &ComponentInteraction) {
    // custom_id format : `sentinel_mod_appeal_{guild_id}_{action_id}`.
    // Le guild_id est numerique (pas d'underscore) et l'action_id est un UUID
    // (tirets, pas d'underscore) -> split_once('_') est sans ambiguite.
    let payload = match component.data.custom_id.strip_prefix(APPEAL_PREFIX) {
        Some(p) => p,
        None => return,
    };
    let (found_guild, action_id) = match payload.split_once('_') {
        Some((g, a)) => (g.to_string(), a),
        // Compat : ancien format sans guild_id embarque -> on ne devine plus le
        // serveur (source du bug multi-guild), on demande /appeal explicite.
        None => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(
                        "Bouton d'appel obsolete. Utilisez `/appeal` dans le serveur concerne.",
                    )
                    .ephemeral(true),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send appeal legacy-button response");
            }
            return;
        }
    };

    let user_id = component.user.id.to_string();
    if !try_start_appeal(&found_guild, &user_id) {
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(
                            "Vous avez déjà soumis un appel récemment. Patientez quelques minutes.",
                        )
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    // Ticket dashboard (best-effort). On lit le client puis on relache le lock.
    if let Some(grpc) = ctx.data.read().await.get::<GrpcClientKey>().cloned() {
        let ticket_req = proto_tickets::CreateTicketRequest {
            title: format!(
                "Appel de sanction — {} (action: {})",
                component.user.name,
                &action_id[..8.min(action_id.len())]
            ),
            priority: "medium".to_string(),
            author_id: component.user.id.to_string(),
            author_name: component.user.name.clone(),
            server: found_guild.clone(),
            category: "appel_sanction".to_string(),
            ticket_type: "appel_sanction".to_string(),
            channel_id: None,
            guild_id: Some(found_guild.clone()),
        };
        if let Err(e) = crate::grpc_call!(@unit &grpc, tickets, create_ticket, ticket_req) {
            warn!(error = %e, "Ticket appel (dashboard) non cree — on continue");
        }
    }

    // `latest` est employe par les sanctions AutoMod, qui ne possedent pas
    // encore d'identifiant d'action individuel mais doivent rester contestables.
    let action_id = (action_id != "latest").then_some(action_id);

    // Repond a l'interaction (differe : la creation du salon peut prendre du temps).
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await;

    finalize_appeal(
        ctx,
        &found_guild,
        component.user.id.get(),
        &component.user.name,
        action_id,
        |content| {
            let ctx = ctx.clone();
            async move {
                let _ = component
                    .create_followup(
                        &ctx.http,
                        serenity::builder::CreateInteractionResponseFollowup::new()
                            .content(content)
                            .ephemeral(true),
                    )
                    .await;
            }
        },
    )
    .await;
    info!(user = %component.user.name, action_id = action_id, "Appel de sanction traite via bouton DM");
}

/// Verifie que le cliqueur est un moderateur (permissions de sanction ou admin).
/// Repond en ephemere et renvoie `false` sinon.
async fn deny_not_mod(ctx: &Context, component: &ComponentInteraction) {
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Réservé aux modérateurs.")
                    .ephemeral(true),
            ),
        )
        .await;
}

pub(crate) async fn ensure_moderator(ctx: &Context, component: &ComponentInteraction) -> bool {
    use serenity::all::Permissions;
    let Some(gid) = component.guild_id else {
        deny_not_mod(ctx, component).await;
        return false;
    };
    let member = match gid.member(&ctx.http, component.user.id).await {
        Ok(m) => m,
        Err(_) => {
            deny_not_mod(ctx, component).await;
            return false;
        }
    };
    #[allow(deprecated)]
    let perms = member
        .permissions(&ctx.cache)
        .unwrap_or_else(|_| Permissions::empty());
    let is_mod = perms.contains(Permissions::ADMINISTRATOR)
        || perms.contains(Permissions::MODERATE_MEMBERS)
        || perms.contains(Permissions::BAN_MEMBERS)
        || perms.contains(Permissions::KICK_MEMBERS)
        || perms.contains(Permissions::MANAGE_GUILD);
    if !is_mod {
        deny_not_mod(ctx, component).await;
    }
    is_mod
}

/// Bouton « Fermer le salon » : supprime le salon d'appel (modo uniquement).
pub async fn handle_appeal_close(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_moderator(ctx, component).await {
        return;
    }
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("🔒 Appel clôturé — suppression du salon…")
                    .ephemeral(true),
            ),
        )
        .await;
    if let Err(e) = component.channel_id.delete(&ctx.http).await {
        warn!(error = %e, "Echec suppression salon d'appel");
    }
}

/// Lit le quorum de votes d'annulation (config, defaut 2).
async fn cancel_quorum(ctx: &Context, guild_id: &str) -> usize {
    let cfg = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(api) => api
                .get_guild_config_for(guild_id, crate::modules::moderation::MODULE_BOT_NAME)
                .await
                .unwrap_or_default(),
            None => return 2,
        }
    };
    cfg.get("appeal_cancel_quorum")
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(2)
}

/// Verifie que le cliqueur est administrateur (permission ADMINISTRATOR).
async fn ensure_admin(ctx: &Context, component: &ComponentInteraction) -> bool {
    use serenity::all::Permissions;
    let Some(gid) = component.guild_id else {
        deny_not_mod(ctx, component).await;
        return false;
    };
    let member = match gid.member(&ctx.http, component.user.id).await {
        Ok(m) => m,
        Err(_) => {
            deny_not_mod(ctx, component).await;
            return false;
        }
    };
    #[allow(deprecated)]
    let perms = member
        .permissions(&ctx.cache)
        .unwrap_or_else(|_| Permissions::empty());
    if perms.contains(Permissions::ADMINISTRATOR) {
        return true;
    }
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Réservé aux administrateurs.")
                    .ephemeral(true),
            ),
        )
        .await;
    false
}

/// Appelle DELETE /api/moderation/actions/{id} (leve la sanction : unban/unmute,
/// annule les rappels, retire l'action). Renvoie Ok si succes.
async fn do_cancel_action(ctx: &Context, action_id: &str) -> Result<(), String> {
    let grpc = ctx
        .data
        .read()
        .await
        .get::<GrpcClientKey>()
        .cloned()
        .ok_or("api indisponible")?;
    let req = proto_mod::CancelActionRequest {
        action_id: action_id.to_string(),
    };
    let resp = crate::grpc_call!(&grpc, moderation, cancel_action, req)?;
    if resp.cancelled {
        Ok(())
    } else {
        Err("annulation refusee (introuvable ou deja levee)".to_string())
    }
}

/// Embed de statut du vote d'annulation.
fn vote_embed(voters: &[String], quorum: usize) -> serenity::builder::CreateEmbed {
    let count = voters.len();
    let reached = count >= quorum;
    let voter_list = if voters.is_empty() {
        "—".to_string()
    } else {
        voters
            .iter()
            .map(|v| format!("<@{v}>"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let status = if reached {
        "✅ Quorum atteint — un **administrateur** peut valider l'annulation."
    } else {
        "🕒 En attente d'autres votes de modérateurs."
    };
    crate::shared::embeds::info_embed("🗳️ Vote d'annulation de la sanction")
        .description(format!(
            "Votes : **{count}/{quorum}**\nOnt voté : {voter_list}\n\n{status}"
        ))
        .timestamp(serenity::model::Timestamp::now())
}

/// Bouton « Voter pour annuler » : un modo ajoute son vote (quorum requis).
pub async fn handle_vote_cancel(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_moderator(ctx, component).await {
        return;
    }
    let Some(action_id) = component
        .data
        .custom_id
        .strip_prefix(APPEAL_VOTE_PREFIX)
        .map(str::to_string)
    else {
        return;
    };
    let Some(guild_id) = component.guild_id.map(|g| g.to_string()) else {
        return;
    };
    let quorum = cancel_quorum(ctx, &guild_id).await;

    let voters: Vec<String> = {
        let mut map = cancel_votes().lock().unwrap();
        let set = map.entry(action_id.clone()).or_default();
        set.insert(component.user.id.to_string());
        set.iter().cloned().collect()
    };

    // Accuse reception (ephemere) puis met a jour l'embed du salon (boutons conserves).
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!(
                        "🗳️ Ton vote est pris en compte ({}/{quorum}).",
                        voters.len()
                    ))
                    .ephemeral(true),
            ),
        )
        .await;
    let _ = component
        .channel_id
        .edit_message(
            &ctx.http,
            component.message.id,
            serenity::builder::EditMessage::new().embed(vote_embed(&voters, quorum)),
        )
        .await;
}

/// Bouton « Valider l'annulation » : ADMIN uniquement, apres le quorum.
pub async fn handle_validate_cancel(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_admin(ctx, component).await {
        return;
    }
    let Some(action_id) = component
        .data
        .custom_id
        .strip_prefix(APPEAL_VALIDATE_PREFIX)
        .map(str::to_string)
    else {
        return;
    };
    let Some(guild_id) = component.guild_id.map(|g| g.to_string()) else {
        return;
    };
    let quorum = cancel_quorum(ctx, &guild_id).await;

    let count = cancel_votes()
        .lock()
        .unwrap()
        .get(&action_id)
        .map(|s| s.len())
        .unwrap_or(0);
    if count < quorum {
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("Quorum non atteint : {count}/{quorum} vote(s) de modérateurs requis avant validation."))
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await;

    match do_cancel_action(ctx, &action_id).await {
        Ok(()) => {
            cancel_votes().lock().unwrap().remove(&action_id);
            let _ = component
                .create_followup(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponseFollowup::new()
                        .content("♻️ **Sanction annulée** (validée par un admin après vote). Suppression du salon…"),
                )
                .await;
            let _ = component.channel_id.delete(&ctx.http).await;
            info!(action_id, admin = %component.user.name, "Sanction annulee apres vote + validation admin");
        }
        Err(e) => {
            let _ = component
                .create_followup(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponseFollowup::new()
                        .content(format!("Échec de l'annulation : {e}")),
                )
                .await;
        }
    }
}

/// Bouton « Fermer + bannir » (etape 1) : demande une confirmation.
pub async fn handle_ban_close(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_moderator(ctx, component).await {
        return;
    }
    let Some(user_id) = component
        .data
        .custom_id
        .strip_prefix(APPEAL_BANCLOSE_PREFIX)
        .map(str::to_string)
    else {
        return;
    };
    let confirm =
        serenity::builder::CreateActionRow::Buttons(vec![serenity::builder::CreateButton::new(
            format!("{APPEAL_BANCONFIRM_PREFIX}{user_id}"),
        )
        .label("⚠️ Confirmer le bannissement")
        .style(serenity::all::ButtonStyle::Danger)]);
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!(
                        "Confirmer : **bannir <@{user_id}> et fermer le salon** ? Action irréversible."
                    ))
                    .components(vec![confirm])
                    .ephemeral(true),
            ),
        )
        .await;
}

/// Bouton « Confirmer le ban » (etape 2) : bannit + ferme le salon.
pub async fn handle_ban_confirm(ctx: &Context, component: &ComponentInteraction) {
    if !ensure_moderator(ctx, component).await {
        return;
    }
    let Some(user_id) = component
        .data
        .custom_id
        .strip_prefix(APPEAL_BANCONFIRM_PREFIX)
        .map(str::to_string)
    else {
        return;
    };
    let Some(gid) = component.guild_id else {
        return;
    };
    let _ = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("🔨 Bannissement… suppression du salon.")
                    .ephemeral(true),
            ),
        )
        .await;
    if let Ok(uid) = user_id.parse::<u64>() {
        if let Err(e) = gid
            .ban_with_reason(
                &ctx.http,
                UserId::new(uid),
                0,
                "Appel refusé — bannissement",
            )
            .await
        {
            warn!(error = %e, user_id, "Echec ban depuis salon d'appel");
        }
    }
    let _ = component.channel_id.delete(&ctx.http).await;
    info!(user_id, mod = %component.user.name, "Appel refuse -> ban + fermeture salon");
}

/// Construit la ligne de bouton "Contester" attachee aux DM de sanction.
///
/// Le `guild_id` est embarque dans le custom_id pour router l'appel vers le BON
/// serveur (fix multi-guild : on ne devine plus via le cache). Format :
/// `sentinel_mod_appeal_{guild_id}_{action_id}`.
pub fn build_appeal_button(guild_id: &str, action_id: &str) -> serenity::builder::CreateActionRow {
    let button = serenity::builder::CreateButton::new(format!(
        "{}{}_{}",
        APPEAL_PREFIX, guild_id, action_id
    ))
    .label("Contester cette sanction")
    .style(serenity::all::ButtonStyle::Secondary);

    serenity::builder::CreateActionRow::Buttons(vec![button])
}
