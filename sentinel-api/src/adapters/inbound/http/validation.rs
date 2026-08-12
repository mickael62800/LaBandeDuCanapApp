//! Validation centralisee des inputs API.
//!
//! Toutes les fonctions retournent `Result<(), DomainError::ValidationError>`.

use sentinel_core::domain::errors::DomainError;
use uuid::Uuid;

// ── Limites ──

/// Longueur max d'un Discord snowflake ID (19 chiffres max).
const MAX_DISCORD_ID_LEN: usize = 20;
/// Longueur max d'un champ "raison" ou "description".
const MAX_REASON_LEN: usize = 2000;
/// Longueur max d'un champ "contenu" (note, message, etc.).
const MAX_CONTENT_LEN: usize = 4000;
/// Longueur max d'un nom d'utilisateur ou de bot.
const MAX_NAME_LEN: usize = 100;
/// Longueur max d'un champ court (config_key, category, action_type, etc.).
const MAX_SHORT_LEN: usize = 200;
/// Longueur max d'un champ titre.
const MAX_TITLE_LEN: usize = 500;
/// Longueur max d'un champ recherche.
const MAX_SEARCH_LEN: usize = 200;

// ── Validateurs de champs ──

/// Valide qu'un Discord ID (guild_id, user_id, channel_id) est un snowflake valide.
/// Format : 17-20 chiffres uniquement.
pub fn validate_discord_id(field: &str, value: &str) -> Result<(), DomainError> {
    if value.is_empty() {
        return Err(DomainError::ValidationError(format!(
            "{field} ne peut pas etre vide"
        )));
    }
    // `len()` (octets) suffit ici : le controle suivant impose des chiffres
    // ASCII, ou un octet vaut exactement un caractere.
    if value.len() > MAX_DISCORD_ID_LEN {
        return Err(DomainError::ValidationError(format!(
            "{field} trop long ({} chars, max {MAX_DISCORD_ID_LEN})",
            value.len()
        )));
    }
    if !value.chars().all(|c| c.is_ascii_digit()) {
        return Err(DomainError::ValidationError(format!(
            "{field} doit etre un ID numerique (snowflake)"
        )));
    }
    Ok(())
}

/// Valide qu'un Discord ID optionnel est valide s'il est present.
pub fn validate_optional_discord_id(
    field: &str,
    value: &Option<String>,
) -> Result<(), DomainError> {
    if let Some(v) = value {
        if !v.is_empty() {
            validate_discord_id(field, v)?;
        }
    }
    Ok(())
}

/// Valide la longueur d'un champ string obligatoire.
///
/// La longueur est comptee en **caracteres**, pas en octets. `str::len()` rend
/// des octets UTF-8 : le contenu est en francais, donc un texte d'accents et de
/// caracteres accentues etait refuse a mi-chemin de la limite annoncee, et le
/// message d'erreur affichait un nombre de « chars » qui n'en etait pas un.
/// Les bornes de la base sont posees en `VARCHAR(n)`, que Postgres compte
/// aussi en caracteres — les deux mesures concordent desormais.
fn validate_string(field: &str, value: &str, max_len: usize) -> Result<(), DomainError> {
    let len = value.chars().count();
    if len > max_len {
        return Err(DomainError::ValidationError(format!(
            "{field} trop long ({len} caracteres, max {max_len})"
        )));
    }
    Ok(())
}

/// Valide la longueur d'un champ string obligatoire et non-vide.
fn validate_required_string(field: &str, value: &str, max_len: usize) -> Result<(), DomainError> {
    if value.is_empty() {
        return Err(DomainError::ValidationError(format!(
            "{field} ne peut pas etre vide"
        )));
    }
    validate_string(field, value, max_len)
}

/// Valide un champ optionnel string.
fn validate_optional_string(
    field: &str,
    value: &Option<String>,
    max_len: usize,
) -> Result<(), DomainError> {
    if let Some(v) = value {
        validate_string(field, v, max_len)?;
    }
    Ok(())
}

// ── Validateurs de champs typés ──

pub fn validate_reason(value: &str) -> Result<(), DomainError> {
    validate_string("reason", value, MAX_REASON_LEN)
}

pub fn validate_content(value: &str) -> Result<(), DomainError> {
    validate_required_string("content", value, MAX_CONTENT_LEN)
}

pub fn validate_name(field: &str, value: &str) -> Result<(), DomainError> {
    validate_string(field, value, MAX_NAME_LEN)
}

pub fn validate_short(field: &str, value: &str) -> Result<(), DomainError> {
    validate_string(field, value, MAX_SHORT_LEN)
}

pub fn validate_title(value: &str) -> Result<(), DomainError> {
    validate_required_string("title", value, MAX_TITLE_LEN)
}

pub fn validate_search(value: &Option<String>) -> Result<(), DomainError> {
    validate_optional_string("search", value, MAX_SEARCH_LEN)
}

/// Parse un identifiant UUID passe en path/query/body. Retourne une
/// `ValidationError` contextualisee si la chaine n'est pas un UUID valide.
/// Remplace le pattern duplique
/// `Uuid::parse_str(...).map_err(|_| ValidationError(...))`.
pub fn parse_uuid(field: &str, value: &str) -> Result<Uuid, DomainError> {
    Uuid::parse_str(value)
        .map_err(|_| DomainError::ValidationError(format!("{field} invalide : {value}")))
}

// ── Validateurs numériques ──

/// Plafond de securite absolu pour un `limit` (defense en profondeur anti-DoS).
/// Aucune requete legitime n'a besoin de plus ; les handlers appliquent en plus
/// leur propre `normalize_limit`/`clamp` bien plus bas.
pub const MAX_QUERY_LIMIT: i64 = 100_000;

/// Valide que limit est dans [0, MAX_QUERY_LIMIT].
pub fn validate_limit(limit: Option<i64>) -> Result<(), DomainError> {
    if let Some(l) = limit {
        if l < 0 {
            return Err(DomainError::ValidationError("limit doit etre >= 0".into()));
        }
        if l > MAX_QUERY_LIMIT {
            return Err(DomainError::ValidationError(format!(
                "limit trop grand (max {MAX_QUERY_LIMIT})"
            )));
        }
    }
    Ok(())
}

/// Valide que offset est >= 0.
pub fn validate_offset(offset: Option<i64>) -> Result<(), DomainError> {
    if let Some(o) = offset {
        if o < 0 {
            return Err(DomainError::ValidationError("offset doit etre >= 0".into()));
        }
    }
    Ok(())
}

/// Valide limit + offset en une fois.
pub fn validate_pagination(limit: Option<i64>, offset: Option<i64>) -> Result<(), DomainError> {
    validate_limit(limit)?;
    validate_offset(offset)?;
    Ok(())
}

// ── Validateurs composites pour les DTOs courants ──

/// Valide les champs communs d'une action de moderation.
pub fn validate_moderation_action(
    guild_id: &str,
    moderator_id: &str,
    target_id: &str,
    reason: &str,
    action_type: &str,
) -> Result<(), DomainError> {
    validate_discord_id("guild_id", guild_id)?;
    validate_discord_id("moderator_id", moderator_id)?;
    validate_discord_id("target_id", target_id)?;
    validate_reason(reason)?;
    validate_short("action_type", action_type)?;
    Ok(())
}

/// Valide un guild_id passe en path parameter.
pub fn validate_guild_id_path(guild_id: &str) -> Result<(), DomainError> {
    validate_discord_id("guild_id", guild_id)
}

/// Valide un couple guild_id + user_id en path parameters.
pub fn validate_guild_user_path(guild_id: &str, user_id: &str) -> Result<(), DomainError> {
    validate_discord_id("guild_id", guild_id)?;
    validate_discord_id("user_id", user_id)?;
    Ok(())
}

/// Valide les champs d'un SetConfigDto.
#[cfg(test)]
#[path = "tests/validation.rs"]
mod tests;

pub fn validate_bot_config(
    guild_id: &str,
    bot_name: &str,
    config_key: &str,
    config_value: &str,
) -> Result<(), DomainError> {
    validate_discord_id("guild_id", guild_id)?;
    validate_short("bot_name", bot_name)?;
    validate_short("config_key", config_key)?;
    validate_string("config_value", config_value, MAX_CONTENT_LEN)?;
    Ok(())
}
