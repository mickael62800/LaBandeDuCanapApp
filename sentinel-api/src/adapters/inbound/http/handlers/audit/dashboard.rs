use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use redis::AsyncCommands;
use uuid::Uuid;

use axum::extract::Query;

use crate::adapters::inbound::http::dto::audit::dashboard::CreateLogDto;
use crate::adapters::inbound::http::dto::audit::dashboard::DashboardInfractionDto;
use crate::adapters::inbound::http::dto::audit::dashboard::DashboardRuleDto;
use crate::adapters::inbound::http::dto::audit::dashboard::DashboardStatsDto;
use crate::adapters::inbound::http::dto::audit::dashboard::GuildDto;
use crate::adapters::inbound::http::dto::audit::dashboard::GuildFilterParams;

use crate::adapters::inbound::http::dto::audit::dashboard::RegisterGuildDto;
use tracing::warn;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::outbound::system::redis_log_stream;
use ops_core::domain::entities::log_entry::LogEntry;

#[derive(serde::Deserialize)]
pub struct GetLogsQuery {
    pub category: String,
    pub limit: Option<usize>,
    pub level: Option<String>,
}

pub async fn get_logs(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<GetLogsQuery>,
) -> Result<Json<Vec<LogEntry>>, ApiError> {
    let limit = q.limit.unwrap_or(200);
    let logs = redis_log_stream::xrevrange_logs(
        &state.redis_client,
        &q.category,
        q.level.as_deref(),
        limit,
    )
    .await;
    Ok(Json(logs))
}

/// GET /api/stats — stats globales pour le dashboard desktop
pub async fn get_dashboard_stats(
    State(state): State<AppState>,
) -> Result<Json<DashboardStatsDto>, ApiError> {
    // Deux domaines, deux crates : le metier Sentinel et la sante des services
    // de la machine. Les reunir ici plutot que dans un service applicatif est
    // ce qui evite a `sentinel-core` de dependre d'`ops-core`.
    let stats = state.audit.stats_uc.get_dashboard_stats().await?;
    let counts = state.ops.service_registry.count_services().await;
    let health = ops_core::domain::entities::services_health::ServicesHealth {
        bots_online: counts.bots_online,
        bots_total: counts.bots_total,
        workers_online: counts.workers_online,
        workers_total: counts.workers_total,
        redis_online: state.ops.service_registry.ping().await,
    };
    Ok(Json(DashboardStatsDto::compose(stats, health)))
}


/// POST /api/logs — écrire un log (utilisé par les bots/workers).
///
/// Strategie de stockage :
/// - **Toujours** XADD sur la stream Redis `logs:{category}` (capacite
///   bornee par STREAM_MAXLEN, eviction auto). C'est la source pour la
///   page "Logs systeme" du panneau web.
/// - **Postgres** uniquement pour `warn` / `error` : forensics long terme,
///   recherche par guild, persistance entre restarts Redis.
pub async fn create_log(
    State(state): State<AppState>,
    Json(dto): Json<CreateLogDto>,
) -> Result<StatusCode, ApiError> {
    let bot_name = dto.bot.unwrap_or_default();
    let category = dto.category.unwrap_or_else(|| {
        sentinel_core::domain::entities::system::config_parsers::default_log_category(&bot_name)
            .to_string()
    });
    let entry = LogEntry {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        level: dto.level.unwrap_or_else(|| "info".to_string()),
        bot: bot_name,
        server: dto.server.unwrap_or_default(),
        message: dto.message,
        category,
        details: dto.details.unwrap_or(serde_json::json!({})),
    };

    // Redis : toujours, avec MAXLEN auto (cap par categorie).
    redis_log_stream::xadd_log(&state.redis_client, &entry).await;

    // Postgres : uniquement warn/error pour la forensique long-terme.
    if matches!(entry.level.as_str(), "warn" | "warning" | "error" | "fatal") {
        state.log_repo.save(&entry).await?;
    }

    state.broadcaster.broadcast(
        "log_entry_created",
        serde_json::json!({
            "level": &entry.level,
            "bot": &entry.bot,
            "message": &entry.message,
            "category": &entry.category,
            "server": &entry.server,
        }),
    );

    Ok(StatusCode::CREATED)
}

/// GET /api/infractions — journal unifie (detections automod + actions moderees)
///
/// Depuis le refactor du panneau web, cet endpoint agrege :
/// - Table `infractions` : detections automatisees (automod texte/image/conduit).
///   Le champ `moderator` y est hardcode a "AutoMod" car la table ne stocke pas
///   l'identite du composant qui a detecte.
/// - Table `moderation_actions` : sanctions prises (warn/mute/ban/unban) avec
///   leur moderator_name reel (bot, worker ou utilisateur humain via le panneau).
///
/// Resultat : le journal affiche maintenant la vraie diversite de moderateurs.
pub async fn get_all_infractions(
    State(state): State<AppState>,
    Query(params): Query<GuildFilterParams>,
) -> Result<Json<Vec<DashboardInfractionDto>>, ApiError> {
    let infractions = match &params.guild_id {
        Some(gid) => {
            let filters =
                sentinel_core::ports::inbound::moderation::manage_infractions::InfractionFilters {
                    user_id: None,
                    action: None,
                    limit: 200,
                    offset: 0,
                };
            state
                .moderation
                .infractions_uc
                .list_infractions(gid, filters)
                .await?
        }
        None => {
            state
                .moderation
                .infractions_uc
                .list_all_infractions(200, 0)
                .await?
        }
    };

    let actions = state
        .moderation
        .moderation_uc
        .list_actions(params.guild_id.as_deref(), 200)
        .await
        .unwrap_or_default();

    let mut merged: Vec<DashboardInfractionDto> = infractions
        .into_iter()
        .map(DashboardInfractionDto::from)
        .chain(actions.into_iter().map(DashboardInfractionDto::from))
        .collect();

    // Tri global par created_at DESC — les deux sources ont deja trie mais le
    // merge les melange.
    merged.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(Json(merged))
}

/// GET /api/rules — règles (filtrable par guild_id)
pub async fn get_all_rules(
    State(state): State<AppState>,
    Query(params): Query<GuildFilterParams>,
) -> Result<Json<Vec<DashboardRuleDto>>, ApiError> {
    let rules = match &params.guild_id {
        Some(gid) => state.moderation.rules_uc.get_rules(gid).await?,
        None => state.moderation.rules_uc.get_all_rules().await?,
    };
    Ok(Json(
        rules.into_iter().map(DashboardRuleDto::from).collect(),
    ))
}

/// PATCH /api/rules/{id} — toggle enabled/disabled
pub async fn toggle_rule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<TogglePayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let enabled = state
        .moderation
        .rules_uc
        .toggle_rule(id, payload.enabled)
        .await?;
    Ok(Json(serde_json::json!({ "enabled": enabled })))
}

#[derive(serde::Deserialize)]
pub struct TogglePayload {
    pub enabled: bool,
}

/// POST /api/bots/heartbeat — un bot signale qu'il est en ligne
pub async fn bot_heartbeat(
    State(state): State<AppState>,
    Json(payload): Json<HeartbeatPayload>,
) -> Result<axum::http::StatusCode, ApiError> {
    // Stocker dans Redis avec TTL de 90 secondes
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        use redis::AsyncCommands;
        let key = format!("bot:online:{}", payload.name);
        if let Err(e) = conn.set_ex::<_, _, ()>(&key, "1", 90).await {
            warn!(error = %e, bot = %payload.name, "Echec Redis set_ex heartbeat");
        }
        // Enregistrer aussi dans l'ensemble des bots connus
        if let Err(e) = conn.sadd::<_, _, ()>("bots:known", &payload.name).await {
            warn!(error = %e, bot = %payload.name, "Echec Redis sadd bots:known");
        }
    }

    state.broadcaster.broadcast(
        "bot_heartbeat",
        serde_json::json!({ "name": &payload.name }),
    );

    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct HeartbeatPayload {
    pub name: String,
}

/// GET /api/guilds — liste des serveurs connus (cache 5min)
pub async fn list_guilds(State(state): State<AppState>) -> Result<Json<Vec<GuildDto>>, ApiError> {
    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(json)) = conn.get::<_, Option<String>>("guilds:all").await {
            if let Ok(dtos) = serde_json::from_str::<Vec<GuildDto>>(&json) {
                return Ok(Json(dtos));
            }
        }
    }

    let guilds = state.system.guild_repo.find_all().await?;
    let dtos: Vec<GuildDto> = guilds.into_iter().map(GuildDto::from).collect();

    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&dtos) {
            if let Err(e) = conn.set_ex::<_, _, ()>("guilds:all", json, 300u64).await {
                warn!(error = %e, "Echec cache set guilds:all");
            }
        }
    }

    Ok(Json(dtos))
}

/// POST /api/guilds/register — un bot enregistre/met à jour un serveur
pub async fn register_guild(
    State(state): State<AppState>,
    Json(dto): Json<RegisterGuildDto>,
) -> Result<StatusCode, ApiError> {
    let guild_id = dto.guild_id.clone();

    let guild = sentinel_core::domain::entities::system::guild::Guild {
        guild_id: dto.guild_id,
        name: dto.name,
        icon: dto.icon,
        member_count: dto.member_count.unwrap_or(0),
        registered_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    state.system.guild_repo.upsert(&guild).await?;

    // Seed des regles de moderation par defaut (idempotent). Couvre les
    // nouvelles guilds + le retro-seed des anciennes au prochain bot startup.
    if let Err(e) = state
        .moderation
        .rules_uc
        .seed_default_rules(&guild_id)
        .await
    {
        warn!(error = %e, guild_id = %guild_id, "Echec seed rules par defaut");
    }

    // Invalider le cache guilds + cache rules de cette guild
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Err(e) = conn.del::<_, ()>("guilds:all").await {
            warn!(error = %e, "Echec invalidation cache guilds:all");
        }
        let _: Result<(), _> = conn.del(format!("rules:{guild_id}")).await;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/guilds/{guild_id} — le bot a ete retire d'un serveur.
/// Supprime la ligne et invalide le cache pour que le selecteur web
/// cesse d'afficher un serveur fantome.
pub async fn delete_guild(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<StatusCode, ApiError> {
    state.system.guild_repo.delete(&guild_id).await?;
    invalidate_guilds_cache(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/guilds/reconcile — le bot envoie au demarrage la liste complete
/// des serveurs dont il fait partie. On supprime tous les autres (cas d'un
/// retrait survenu pendant que le bot etait hors ligne, ou jamais nettoye).
pub async fn reconcile_guilds(
    State(state): State<AppState>,
    Json(dto): Json<ReconcileGuildsDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deleted = state
        .system
        .guild_repo
        .delete_absent(&dto.guild_ids)
        .await?;
    if deleted > 0 {
        invalidate_guilds_cache(&state).await;
    }
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

#[derive(serde::Deserialize)]
pub struct ReconcileGuildsDto {
    pub guild_ids: Vec<String>,
}

/// Invalide le cache Redis `guilds:all`.
async fn invalidate_guilds_cache(state: &AppState) {
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Err(e) = conn.del::<_, ()>("guilds:all").await {
            warn!(error = %e, "Echec invalidation cache guilds:all");
        }
    }
}

#[cfg(test)]
#[path = "tests/dashboard.rs"]
mod tests;
