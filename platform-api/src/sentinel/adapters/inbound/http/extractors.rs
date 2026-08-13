//! Extractors Axum qui valident les path parameters Discord (guild_id,
//! user_id) AVANT d'entrer dans le handler.
//!
//! Avant : chaque handler extrayait `Path<(String, String)>` puis appelait
//! (ou oubliait d'appeler) `validation::validate_guild_user_path`. Ces
//! extractors centralisent la validation : impossible d'entrer dans le
//! handler avec un id invalide, et plus aucun appel a recopier.

use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;

use platform_core::sentinel::domain::errors::DomainError;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::validation;

/// Path `{guild_id}` valide comme un identifiant Discord.
pub struct ValidatedGuild {
    pub guild_id: String,
}

impl<S: Send + Sync> FromRequestParts<S> for ValidatedGuild {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(guild_id) = Path::<String>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError(DomainError::ValidationError("guild_id manquant".into())))?;
        validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;
        Ok(Self { guild_id })
    }
}

/// Path `{guild_id}/{user_id}` valides comme des identifiants Discord.
pub struct ValidatedGuildUser {
    pub guild_id: String,
    pub user_id: String,
}

impl<S: Send + Sync> FromRequestParts<S> for ValidatedGuildUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path((guild_id, user_id)) = Path::<(String, String)>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                ApiError(DomainError::ValidationError(
                    "guild_id/user_id manquants".into(),
                ))
            })?;
        validation::validate_guild_user_path(&guild_id, &user_id).map_err(ApiError)?;
        Ok(Self { guild_id, user_id })
    }
}
