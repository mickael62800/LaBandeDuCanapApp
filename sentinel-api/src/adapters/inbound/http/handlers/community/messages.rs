//! Envoi d'un message texte par le bot dans un salon.
//!
//! Le pendant depouille du builder d'embeds : ici il n'y a rien a construire,
//! seulement du markdown a transmettre. Rien n'est persiste — un message
//! envoye appartient a Discord, le repliquer en base creerait deux verites
//! dont l'une serait fausse des la premiere edition manuelle.
//!
//! L'API ne parle pas a Discord elle-meme : elle depose l'ordre sur le stream
//! et le bot poste. C'est lui qui porte l'identite et qui encaisse deja les
//! rate-limits.

use axum::extract::{Path, State};
use axum::Json;
use redis::AsyncCommands;
use serde::Deserialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::bootstrap::state::CommunityState;
use sentinel_core::domain::errors::DomainError;

const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;

/// Limite Discord. Verifiee ici pour rendre l'erreur AU MOMENT DE L'ENVOI,
/// dans le navigateur : passe le stream, l'echec serait silencieux et
/// l'utilisateur croirait son message parti.
const MAX_CONTENT: usize = 2000;

#[derive(Debug, Deserialize)]
pub struct SendMessageDto {
    pub content: String,
    /// URL ABSOLUE d'une image a joindre (facultatif). Le bot la telecharge et
    /// la poste en piece jointe. Un message avec image seule (sans texte) est
    /// permis.
    #[serde(default)]
    pub image_url: Option<String>,
}

/// POST /api/messages/{guild_id}/{channel_id}
pub async fn send_message(
    State(state): State<CommunityState>,
    Path((guild_id, channel_id)): Path<(String, String)>,
    Json(dto): Json<SendMessageDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let content = dto.content.trim();
    let image_url = dto
        .image_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    // Un message est valide s'il porte du texte OU une image. Rejeter le vide
    // ici (plutot que de laisser le stream avaler un ordre sans effet) rend
    // l'erreur au moment de l'envoi, dans le navigateur.
    if content.is_empty() && image_url.is_none() {
        return Err(ApiError(DomainError::ValidationError(
            "le message est vide (ni texte ni image)".into(),
        )));
    }
    // En CARACTERES, pas en octets : Discord compte des caracteres, et un
    // message d'emojis serait refuse bien avant 2000 avec `len()`.
    let taille = content.chars().count();
    if taille > MAX_CONTENT {
        return Err(ApiError(DomainError::ValidationError(format!(
            "message trop long : {taille} caracteres, maximum {MAX_CONTENT}"
        ))));
    }

    let envelope = serde_json::json!({
        "event": "message_send",
        "data": {
            "guild_id": guild_id,
            "channel_id": channel_id,
            "content": content,
            "image_url": image_url,
        },
    })
    .to_string();

    let mut conn = state
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("Redis indisponible: {e}"))))?;
    let _: String = conn
        .xadd_maxlen(
            STREAM_KEY,
            redis::streams::StreamMaxlen::Approx(STREAM_MAXLEN),
            "*",
            &[("payload", envelope)],
        )
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("XADD message_send: {e}"))))?;

    // `queued`, pas `sent` : le bot n'a pas encore poste. Annoncer un envoi
    // reussi ici mentirait si le bot n'a pas acces au salon.
    Ok(Json(serde_json::json!({ "queued": true })))
}
