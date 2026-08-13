use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use redis::AsyncCommands;

use tracing::warn;

use crate::sentinel::adapters::inbound::http::dto::system::bot_config::BotDefinitionDto;
use crate::sentinel::adapters::inbound::http::dto::system::bot_config::BotGuildConfigDto;
use crate::sentinel::adapters::inbound::http::dto::system::bot_config::DeleteConfigDto;
use crate::sentinel::adapters::inbound::http::dto::system::bot_config::SetConfigDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::validation;
use crate::sentinel::bootstrap::state::SystemState;

const DEFINITIONS_TTL: u64 = 3600; // 1 heure
const GUILD_CONFIG_TTL: u64 = 900; // 15 minutes

/// GET /api/bots/definitions — liste des bots et leurs parametres disponibles (cache 1h)
pub async fn get_definitions(
    State(state): State<SystemState>,
) -> Result<Json<Vec<BotDefinitionDto>>, ApiError> {
    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(json)) = conn.get::<_, Option<String>>("bot:definitions").await {
            if let Ok(dtos) = serde_json::from_str::<Vec<BotDefinitionDto>>(&json) {
                return Ok(Json(dtos));
            }
        }
    }

    let defs = state.bot_config_repo.get_definitions().await?;
    let dtos: Vec<BotDefinitionDto> = defs.into_iter().map(BotDefinitionDto::from).collect();

    // Populate cache
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&dtos) {
            if let Err(e) = conn
                .set_ex::<_, _, ()>("bot:definitions", json, DEFINITIONS_TTL)
                .await
            {
                warn!(error = %e, "Echec cache set bot:definitions");
            }
        }
    }

    Ok(Json(dtos))
}

/// GET /api/bots/config/{guild_id} — config de tous les bots pour un serveur (cache 15min)
pub async fn get_guild_config(
    State(state): State<SystemState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<BotGuildConfigDto>>, ApiError> {
    // Validation

    let cache_key = format!("bot:config:{guild_id}");

    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(json)) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Ok(dtos) = serde_json::from_str::<Vec<BotGuildConfigDto>>(&json) {
                return Ok(Json(dtos));
            }
        }
    }

    let configs = state.bot_config_repo.get_all_config(&guild_id).await?;
    let dtos: Vec<BotGuildConfigDto> = configs.into_iter().map(BotGuildConfigDto::from).collect();

    // Populate cache
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&dtos) {
            if let Err(e) = conn
                .set_ex::<_, _, ()>(&cache_key, json, GUILD_CONFIG_TTL)
                .await
            {
                warn!(error = %e, cache_key = %cache_key, "Echec cache set guild config");
            }
        }
    }

    Ok(Json(dtos))
}

/// GET /api/bots/config/{guild_id}/{bot_name} — config d'un bot specifique pour un serveur
pub async fn get_bot_config(
    State(state): State<SystemState>,
    Path((guild_id, bot_name)): Path<(String, String)>,
) -> Result<Json<Vec<BotGuildConfigDto>>, ApiError> {
    // Validation
    validation::validate_short("bot_name", &bot_name).map_err(ApiError)?;

    let cache_key = format!("bot:config:{guild_id}:{bot_name}");

    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(json)) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Ok(dtos) = serde_json::from_str::<Vec<BotGuildConfigDto>>(&json) {
                return Ok(Json(dtos));
            }
        }
    }

    let configs = state
        .bot_config_repo
        .get_config(&guild_id, &bot_name)
        .await?;
    let dtos: Vec<BotGuildConfigDto> = configs.into_iter().map(BotGuildConfigDto::from).collect();

    // Populate cache
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&dtos) {
            if let Err(e) = conn
                .set_ex::<_, _, ()>(&cache_key, json, GUILD_CONFIG_TTL)
                .await
            {
                warn!(error = %e, cache_key = %cache_key, "Echec cache set bot config");
            }
        }
    }

    Ok(Json(dtos))
}

/// POST /api/bots/config — sauvegarder un parametre + invalider le cache
pub async fn set_config(
    State(state): State<SystemState>,
    Json(dto): Json<SetConfigDto>,
) -> Result<StatusCode, ApiError> {
    // Validation
    validation::validate_bot_config(
        &dto.guild_id,
        &dto.bot_name,
        &dto.config_key,
        &dto.config_value,
    )
    .map_err(ApiError)?;

    // Phase 7 B — Gate RBAC : admin+ requis pour modifier la config bot.
    // Body-based -> check_role_for_guild (lookup DB explicite + distingue
    // les vraies erreurs DB des refus de role, post-fix P0.C).

    state
        .bot_config_repo
        .set_config(
            &dto.guild_id,
            &dto.bot_name,
            &dto.config_key,
            &dto.config_value,
        )
        .await?;

    // Invalider les caches
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Err(e) = conn
            .del::<_, ()>(format!("bot:config:{}", dto.guild_id))
            .await
        {
            warn!(error = %e, guild_id = %dto.guild_id, "Echec invalidation cache config guild");
        }
        if let Err(e) = conn
            .del::<_, ()>(format!("bot:config:{}:{}", dto.guild_id, dto.bot_name))
            .await
        {
            warn!(error = %e, guild_id = %dto.guild_id, "Echec invalidation cache config bot");
        }
    }

    // Toggle module : broadcast pour que le bot re-register les slash
    // commands de cette guild. Cache/decache les commandes Discord
    // instantanement (au lieu d'attendre un restart).
    if dto.config_key == "enabled" {
        let enabled =
            platform_core::sentinel::domain::entities::system::config_parsers::parse_bool_str(
                &dto.config_value,
            );
        state.broadcaster.broadcast(
            "bot_enabled_changed",
            serde_json::json!({
                "guild_id": &dto.guild_id,
                "bot_name": &dto.bot_name,
                "enabled": enabled,
            }),
        );
    }

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/bots/config — supprimer un parametre + invalider le cache
pub async fn delete_config(
    State(state): State<SystemState>,
    Json(dto): Json<DeleteConfigDto>,
) -> Result<StatusCode, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_short("bot_name", &dto.bot_name).map_err(ApiError)?;
    validation::validate_short("config_key", &dto.config_key).map_err(ApiError)?;

    // Phase 7 B — Gate RBAC : admin+ requis pour supprimer une cle de config.

    state
        .bot_config_repo
        .delete_config(&dto.guild_id, &dto.bot_name, &dto.config_key)
        .await?;

    // Invalider les caches
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Err(e) = conn
            .del::<_, ()>(format!("bot:config:{}", dto.guild_id))
            .await
        {
            warn!(error = %e, guild_id = %dto.guild_id, "Echec invalidation cache config guild");
        }
        if let Err(e) = conn
            .del::<_, ()>(format!("bot:config:{}:{}", dto.guild_id, dto.bot_name))
            .await
        {
            warn!(error = %e, guild_id = %dto.guild_id, "Echec invalidation cache config bot");
        }
    }

    // Si suppression de la cle "enabled" : retour au default true ->
    // re-register pour que les commandes du module reapparaissent.
    if dto.config_key == "enabled" {
        state.broadcaster.broadcast(
            "bot_enabled_changed",
            serde_json::json!({
                "guild_id": &dto.guild_id,
                "bot_name": &dto.bot_name,
                "enabled": true,
            }),
        );
    }

    Ok(StatusCode::NO_CONTENT)
}
