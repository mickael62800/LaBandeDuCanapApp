//! Factory reset d'un serveur (DANGER, IRREVERSIBLE).
//!
//! `POST /api/system/guild-reset/{guild_id}` — reserve a l'OWNER (user) avec
//! une confirmation forte (le nom exact du serveur). Efface toutes les donnees
//! du guild en base, puis publie un event Redis `guild_reset` pour que le bot
//! annule l'etat Discord (deban / unmute / retrait des roles temp+quarantaine).

use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::State;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::event_signing;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::SystemState;

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct ResetGuildBody {
    /// Doit etre EXACTEMENT le nom du serveur (garde-fou anti-clic accidentel).
    pub confirmation: String,
    /// Actions Discord a executer par le bot (toutes activees par defaut).
    #[serde(default = "default_true")]
    pub unban: bool,
    #[serde(default = "default_true")]
    pub unmute: bool,
    #[serde(default = "default_true")]
    pub remove_roles: bool,
}

#[derive(Debug, Serialize)]
pub struct ResetGuildResponse {
    pub tables_wiped: usize,
    pub total_rows: u64,
}

/// POST /api/system/guild-reset/{guild_id}
pub async fn reset_guild(
    State(state): State<SystemState>,
    user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<ResetGuildBody>,
) -> Result<Json<ResetGuildResponse>, ApiError> {
    // Acteur du reset pour la trace. `None` = appel interne (bot/worker) :
    // l'acces est deja filtre en amont par le gate superadmin.
    let actor = user
        .map(|e| e.0.discord_user_id)
        .unwrap_or_else(|| "internal".to_string());

    // ── Garde-fou : confirmation forte (nom du serveur), verifiee cote use case ──
    let outcome = state
        .reset_guild_uc
        .reset(&guild_id, &body.confirmation)
        .await?;

    tracing::warn!(
        guild_id,
        actor = %actor,
        total_rows = outcome.total_rows,
        tables = outcome.tables_wiped.len(),
        "FACTORY RESET execute (donnees du serveur effacees)"
    );

    // ── Event vers le bot : annule l'etat Discord ──
    // Signature HMAC (secret = API_KEY partage bot<->api) : le bot rejette un
    // event guild_reset non signe ou mal signe -> impossible de forcer un reset
    // destructif (unban-all + strip-roles) en publiant sur Redis sans le secret.
    let sig = sign_guild_reset(
        &state.api_key,
        &guild_id,
        body.unban,
        body.unmute,
        body.remove_roles,
    );
    state.broadcaster.broadcast(
        "guild_reset",
        serde_json::json!({
            "guild_id": guild_id,
            "unban": body.unban,
            "unmute": body.unmute,
            "remove_roles": body.remove_roles,
            "quarantine_role_id": outcome.discord_context.quarantine_role_id,
            "temp_role_ids": outcome.discord_context.temp_role_ids,
            "actor": { "source": "web", "user_id": actor },
            "sig": sig,
        }),
    );

    Ok(Json(ResetGuildResponse {
        tables_wiped: outcome.tables_wiped.len(),
        total_rows: outcome.total_rows,
    }))
}

/// Signature HMAC-SHA256 d'un event `guild_reset`.
///
/// Le HMAC et le message canonique vivent desormais dans
/// [`crate::sentinel::adapters::inbound::http::event_signing`], partages avec les events
/// `guild_backup:*` qui sont tout aussi destructifs.
pub fn sign_guild_reset(
    secret: &str,
    guild_id: &str,
    unban: bool,
    unmute: bool,
    remove_roles: bool,
) -> String {
    event_signing::sign(
        secret,
        &event_signing::guild_reset_message(guild_id, unban, unmute, remove_roles),
    )
}
