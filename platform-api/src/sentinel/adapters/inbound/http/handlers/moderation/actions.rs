use crate::sentinel::adapters::inbound::http::extractors::{ValidatedGuild, ValidatedGuildUser};
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;

use crate::sentinel::adapters::inbound::http::dto::moderation::actions::BanEntryDto;
use crate::sentinel::adapters::inbound::http::dto::moderation::actions::LogActionDto;
use crate::sentinel::adapters::inbound::http::dto::moderation::actions::ModerationActionResponseDto;
use crate::sentinel::adapters::inbound::http::dto::moderation::actions::UserHistoryDto;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::ok_response;
use crate::sentinel::adapters::inbound::http::helpers::single_dto;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::adapters::inbound::http::validation;
use crate::sentinel::bootstrap::state::ModerationState;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;

/// S1/S4 — Resout l'identite moderateur a journaliser.
///
/// Web (WebUser present, token Discord verifie) -> identite authentifiee,
/// les valeurs eventuellement fournies dans le body sont ignorees. Interne
/// (pas de WebUser : gRPC/Bearer/desktop) -> valeurs par defaut fournies.
fn resolve_web_moderator(
    user: &Option<Extension<WebUser>>,
    default_id: &str,
    default_name: &str,
) -> (String, String) {
    match user {
        Some(Extension(ctx)) => (ctx.discord_user_id.clone(), ctx.discord_user_id.clone()),
        None => (default_id.to_string(), default_name.to_string()),
    }
}

#[derive(Debug, Deserialize)]
pub struct BansQuery {
    pub guild_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

mod evidence;
mod execution;
mod history;
mod reviews;
mod stats;

pub use evidence::*;
pub use execution::*;
pub use history::*;
pub use reviews::*;
pub use stats::*;

#[cfg(test)]
use reviews::review_entry_to_dto;

#[cfg(test)]
#[path = "tests/actions.rs"]
mod tests;
