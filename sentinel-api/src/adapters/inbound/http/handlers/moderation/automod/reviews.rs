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

use crate::adapters::inbound::http::dto::moderation::infractions::InfractionResponseDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::normalize_limit;
use crate::adapters::inbound::http::helpers::normalize_offset;
use crate::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::adapters::inbound::http::validation;
use crate::bootstrap::state::ModerationState;
use sentinel_core::domain::entities::moderation::review::automod::AutomodReview;
use sentinel_core::domain::entities::moderation::review::automod::ModeratorFacts;
use sentinel_core::domain::entities::moderation::review::automod::NewAutomodReview;
use sentinel_core::domain::entities::moderation::review::automod::SuggestedAction;
use sentinel_core::domain::entities::system::discord_ids::ChannelId;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
use sentinel_core::domain::entities::system::discord_ids::MessageId;
use sentinel_core::domain::entities::system::discord_ids::UserId;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::inbound::moderation::manage_automod_reviews::ResolveAutomodReviewCommand;
use sentinel_core::ports::inbound::moderation::manage_infractions::InfractionFilters;

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
