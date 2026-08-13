//! Handlers HTTP des cartes de review automod (detections, votes, resolution).
//!
//! Pas de logique metier ici — on reutilise `ManageInfractionsUseCase`
//! (port inbound) avec un filtre `action="detection"`. La page
//! `/automod` cote web consomme ce endpoint pour la timeline des
//! detections automod.

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::sentinel::adapters::inbound::http::dto::moderation::infractions::InfractionResponseDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::normalize_limit;
use crate::sentinel::adapters::inbound::http::helpers::normalize_offset;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::adapters::inbound::http::validation;
use crate::sentinel::bootstrap::state::ModerationState;
use platform_core::sentinel::domain::entities::moderation::review::automod::AutomodReview;
use platform_core::sentinel::domain::entities::moderation::review::automod::ModeratorFacts;
use platform_core::sentinel::domain::entities::moderation::review::automod::NewAutomodReview;
use platform_core::sentinel::domain::entities::moderation::review::automod::SuggestedAction;
use platform_core::sentinel::domain::entities::system::discord_ids::ChannelId;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::MessageId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;
use platform_core::sentinel::domain::enums::system::role::Role;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::moderation::manage_automod_reviews::ResolveAutomodReviewCommand;
use platform_core::sentinel::ports::inbound::moderation::manage_infractions::InfractionFilters;

use super::dto::AutomodReviewDto;
use super::dto::ReviewVoteDto;

mod creation;
mod lifecycle;
mod queries;
mod resolution;
mod voting;

pub use creation::*;
pub use lifecycle::*;
pub use queries::*;
pub use resolution::*;
pub use voting::*;
