//! Module welcome — bienvenue/depart (ex welcome-bot).

pub const MODULE_BOT_NAME: &str = "welcome-bot";

pub mod api_client;
pub mod ghost;
pub mod handler;
pub mod template;

use serenity::all::{ComponentInteraction, Context, Member};
use serenity::model::id::GuildId;

pub async fn on_member_add(ctx: &Context, member: &Member) {
    handler::on_member_add(ctx, member).await;
}

pub async fn on_member_remove(
    ctx: &Context,
    guild_id: GuildId,
    user: &serenity::model::user::User,
) {
    handler::on_member_remove(ctx, guild_id, user).await;
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    handler::on_component(ctx, component).await;
}

/// Fin du filtrage d'adhesion natif Discord (membership screening) : on
/// attribue le(s) role(s) du reglement, comme via le bouton du bot.
pub async fn on_screening_complete(
    ctx: &Context,
    guild_id: GuildId,
    user_id: serenity::model::id::UserId,
) {
    handler::on_screening_complete(ctx, guild_id, user_id).await;
}

pub async fn on_voice_state_update(
    ctx: &Context,
    old: &Option<serenity::model::voice::VoiceState>,
    new: &serenity::model::voice::VoiceState,
) {
    handler::on_voice_state_update(ctx, old, new).await;
}

pub fn handles_component(custom_id: &str) -> bool {
    custom_id == handler::RULES_ACCEPT_ID
}

pub fn handles_modal(custom_id: &str) -> bool {
    custom_id == handler::AGE_MODAL_ID
}

/// Verification d'age active sur la guild ? (pour suspendre les auto-roles).
pub async fn age_check_active(ctx: &Context, guild_id: GuildId) -> bool {
    handler::age_check_active(ctx, guild_id).await
}

pub async fn on_modal(ctx: &Context, modal: &serenity::model::application::ModalInteraction) {
    handler::handle_age_modal(ctx, modal).await;
}

/// Spawn le consumer durable (Redis stream). Appele une fois au `ready`.
/// Ecoute `welcome_rules_publish` (bouton "Publier le reglement" du dashboard)
/// et poste le panneau de reglement avec le bouton d'acceptation.
pub fn spawn(ctx: Context) {
    // Garde run-once : `ready` refire a chaque reconnexion gateway -> sans garde,
    // N consumers Redis s'accumulent (panneau de reglement publie N fois).
    use std::sync::atomic::{AtomicBool, Ordering};
    static SPAWNED: AtomicBool = AtomicBool::new(false);
    if SPAWNED.swap(true, Ordering::SeqCst) {
        return;
    }

    // Boucle de rafraichissement des compteurs (membres + vocal). Les events
    // join/leave ne suffisent pas : un compteur active alors que personne ne
    // bouge ne se mettrait jamais a jour. On repasse periodiquement sur chaque
    // guild. `update_counter` ne renomme que si le nombre a change (rate limit).
    {
        let ctx = ctx.clone();
        tokio::spawn(async move {
            // Laisse le cache (membres, voice states) se peupler apres le boot.
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            loop {
                for guild_id in ctx.cache.guilds() {
                    handler::refresh_counters(&ctx, guild_id).await;
                }
                // 10 min : sous la limite Discord de 2 renommages / 10 min / salon.
                tokio::time::sleep(std::time::Duration::from_secs(600)).await;
            }
        });
    }

    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "sentinel-bot-welcome".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move { handle_event(&ctx, &payload_json).await }
            },
        )
        .await;
    });
}

async fn handle_event(ctx: &Context, payload_json: &str) {
    let envelope: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    let event = envelope.get("event").and_then(|v| v.as_str());
    let data = envelope.get("data");
    let guild_id = data
        .and_then(|d| d.get("guild_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok());

    match event {
        Some("welcome_rules_publish") => {
            if let Some(g) = guild_id {
                if let Err(e) = handler::publish_rules_panel(ctx, GuildId::new(g)).await {
                    tracing::warn!(error = %e, guild = g, "Echec publication panneau reglement");
                }
            }
        }
        // Deban d'un membre dont le ban de verification d'age est echu
        // (emis par le worker age_unban).
        Some("age_ban_lift") => {
            let user_id = data
                .and_then(|d| d.get("user_id"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok());
            if let (Some(g), Some(u)) = (guild_id, user_id) {
                handler::lift_age_ban(ctx, GuildId::new(g), u).await;
            }
        }
        _ => {}
    }
}
