//! Endpoints PUBLICS du site communautaire — accessibles SANS connexion.
//!
//! # Regle de securite
//!
//! Ces handlers sont montes en dehors de toute la pile d'authentification
//! (`auth_middleware`, `guild_auth`, `rbac`, `whitelist`). Chaque champ expose
//! ici est donc lisible par n'importe qui sur Internet.
//!
//! Consequence pratique : on ne renvoie JAMAIS d'entite du domaine telle
//! quelle. Chaque DTO public est ecrit a la main, champ par champ, en partant
//! de rien. Reutiliser un DTO interne exposerait au premier ajout de colonne
//! une donnee qu'on n'avait pas l'intention de publier.
//!
//! Aucune donnee personnelle : pas d'ID Discord d'utilisateur, pas de pseudo,
//! pas de log, pas de sanction.

use axum::extract::{Path, State};
use axum::Json;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::bootstrap::state::SystemState;
use platform_core::sentinel::domain::errors::DomainError;
use serde::Serialize;

/// Vitrine d'un serveur : ce qu'un visiteur non connecte peut voir.
#[derive(Debug, Serialize)]
pub struct PublicGuildDto {
    pub guild_id: String,
    pub name: String,
    /// Hash d'icone Discord ; le front construit l'URL du CDN.
    pub icon: Option<String>,
    pub member_count: i32,
}

/// GET /api/public/guilds/{guild_id}
pub async fn public_guild(
    State(state): State<SystemState>,
    Path(guild_id): Path<String>,
) -> Result<Json<PublicGuildDto>, ApiError> {
    // Validation stricte : un identifiant non numerique n'atteint pas la base.
    // Endpoint non authentifie, donc expose au balayage automatise.
    if guild_id.len() > 20 || !guild_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError(DomainError::ValidationError(
            "guild_id invalide".into(),
        )));
    }

    let guild = state
        .guild_repo
        .find_by_id(&guild_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("serveur introuvable".into()))?;

    Ok(Json(PublicGuildDto {
        guild_id: guild.guild_id.into_inner(),
        name: guild.name,
        icon: guild.icon,
        member_count: guild.member_count,
    }))
}
