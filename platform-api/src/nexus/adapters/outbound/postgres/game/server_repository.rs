use crate::nexus::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use platform_core::nexus::domain::entities::game::server::{GameServer, GameServerStatus};
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::game_server_repository::{
    GameServerRepository, GameServerRuntimeUpdate, NewGameServer, TemplateUsage,
};

pub struct PgGameServerRepository {
    pool: PgPool,
}

impl PgGameServerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct ServerRow {
    id: Uuid,
    guild_id: String,
    template_id: Uuid,
    name: String,
    status: String,
    container_id: Option<String>,
    container_name: Option<String>,
    host_port: Option<i32>,
    rcon_port: Option<i32>,
    rcon_password: Option<String>,
    volume_name: Option<String>,
    allocated_memory_mb: i32,
    cpu_limit: Option<f64>,
    owner_user_id: String,
    idle_shutdown_days: Option<i32>,
    last_active_at: Option<DateTime<Utc>>,
    last_player_count: i32,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    stopped_at: Option<DateTime<Utc>>,
    restart_attempts: i32,
    last_restart_at: Option<DateTime<Utc>>,
    text_channel_id: Option<String>,
    voice_channel_id: Option<String>,
    ip_reveal_at: Option<DateTime<Utc>>,
    closes_at: Option<DateTime<Utc>>,
    config_dirty: bool,
    rcon_latency_ms: Option<i32>,
    net_rx_bytes: Option<i64>,
    net_tx_bytes: Option<i64>,
    net_sampled_at: Option<DateTime<Utc>>,
    ip_revealed: bool,
}

impl TryFrom<ServerRow> for GameServer {
    type Error = DomainError;
    fn try_from(r: ServerRow) -> Result<Self, DomainError> {
        let status = GameServerStatus::from_str(&r.status)
            .ok_or_else(|| DomainError::Internal(format!("status invalide: {}", r.status)))?;
        let host_port = r.host_port.and_then(|p| u16::try_from(p).ok());
        let rcon_port = r.rcon_port.and_then(|p| u16::try_from(p).ok());
        Ok(GameServer {
            id: r.id,
            guild_id: r.guild_id,
            template_id: r.template_id,
            name: r.name,
            status,
            container_id: r.container_id,
            container_name: r.container_name,
            host_port,
            rcon_port,
            rcon_password: r.rcon_password,
            volume_name: r.volume_name,
            allocated_memory_mb: r.allocated_memory_mb,
            cpu_limit: r.cpu_limit,
            owner_user_id: r.owner_user_id,
            idle_shutdown_days: r.idle_shutdown_days,
            last_active_at: r.last_active_at,
            last_player_count: r.last_player_count,
            last_error: r.last_error,
            created_at: r.created_at,
            updated_at: r.updated_at,
            started_at: r.started_at,
            stopped_at: r.stopped_at,
            restart_attempts: r.restart_attempts,
            last_restart_at: r.last_restart_at,
            text_channel_id: r.text_channel_id,
            voice_channel_id: r.voice_channel_id,
            ip_reveal_at: r.ip_reveal_at,
            closes_at: r.closes_at,
            config_dirty: r.config_dirty,
            rcon_latency_ms: r.rcon_latency_ms,
            net_rx_bytes: r.net_rx_bytes,
            net_tx_bytes: r.net_tx_bytes,
            net_sampled_at: r.net_sampled_at,
            ip_revealed: r.ip_revealed,
        })
    }
}

const SELECT_COLS: &str = "id, guild_id, template_id, name, status, container_id, container_name, \
     host_port, rcon_port, rcon_password, volume_name, allocated_memory_mb, cpu_limit, \
     owner_user_id, idle_shutdown_days, last_active_at, last_player_count, \
     last_error, created_at, updated_at, started_at, stopped_at, \
     restart_attempts, last_restart_at, \
     text_channel_id, voice_channel_id, ip_reveal_at, ip_revealed, closes_at, config_dirty,      rcon_latency_ms, net_rx_bytes, net_tx_bytes, net_sampled_at";

#[async_trait]
impl GameServerRepository for PgGameServerRepository {
    async fn create(&self, new: NewGameServer) -> Result<GameServer, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_ctx("tx begin"))?;

        let row: ServerRow = sqlx::query_as(&format!(
            "INSERT INTO game_servers \
                 (guild_id, template_id, name, allocated_memory_mb, cpu_limit, owner_user_id, idle_shutdown_days) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             RETURNING {SELECT_COLS}"
        ))
        .bind(new.guild_id.as_str())
        .bind(new.template_id)
        .bind(&new.name)
        .bind(new.allocated_memory_mb)
        .bind(new.cpu_limit)
        .bind(&new.owner_user_id)
        .bind(new.idle_shutdown_days)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_ctx("create game_server"))?;

        for (key, value) in &new.initial_config {
            sqlx::query(
                "INSERT INTO game_server_configs (server_id, config_key, config_value, updated_by) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(row.id)
            .bind(key)
            .bind(value)
            .bind(&new.owner_user_id)
            .execute(&mut *tx)
            .await
            .map_err(pg_ctx("create game_server_configs"))?;
        }

        tx.commit().await.map_err(pg_ctx("tx commit"))?;
        GameServer::try_from(row)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<GameServer>, DomainError> {
        let row: Option<ServerRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM game_servers WHERE id = $1 AND deleted_at IS NULL AND status != 'deleted'"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("find game_server"))?;
        row.map(GameServer::try_from).transpose()
    }

    async fn find_existing_ids(&self, ids: &[Uuid]) -> Result<HashSet<Uuid>, DomainError> {
        if ids.is_empty() {
            return Ok(HashSet::new());
        }

        let rows = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM game_servers \
             WHERE id = ANY($1) AND deleted_at IS NULL AND status != 'deleted'",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("find existing game_server ids"))?;

        Ok(rows.into_iter().collect())
    }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<GameServer>, DomainError> {
        let rows: Vec<ServerRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM game_servers \
             WHERE guild_id = $1 AND deleted_at IS NULL AND status != 'deleted' ORDER BY created_at DESC"
        ))
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("list game_servers"))?;
        rows.into_iter().map(GameServer::try_from).collect()
    }

    async fn list_running(&self) -> Result<Vec<GameServer>, DomainError> {
        let rows: Vec<ServerRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM game_servers \
             WHERE status = 'running' AND deleted_at IS NULL"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("list running game_servers"))?;
        rows.into_iter().map(GameServer::try_from).collect()
    }

    async fn list_active(&self) -> Result<Vec<GameServer>, DomainError> {
        let rows: Vec<ServerRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM game_servers \
             WHERE status IN ('starting', 'running', 'stopping') AND deleted_at IS NULL"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("list active game_servers"))?;
        rows.into_iter().map(GameServer::try_from).collect()
    }

    async fn update_runtime(
        &self,
        id: Uuid,
        update: GameServerRuntimeUpdate,
    ) -> Result<(), DomainError> {
        // Construction dynamique des SET clauses pour ne toucher que les champs presents.
        let mut sets: Vec<String> = Vec::new();
        let mut idx = 2; // $1 = id

        if update.status.is_some() {
            sets.push(format!("status = ${}", idx));
            idx += 1;
        }
        if update.container_id.is_some() {
            sets.push(format!("container_id = ${}", idx));
            idx += 1;
        }
        if update.container_name.is_some() {
            sets.push(format!("container_name = ${}", idx));
            idx += 1;
        }
        if update.host_port.is_some() {
            sets.push(format!("host_port = ${}", idx));
            idx += 1;
        }
        if update.rcon_port.is_some() {
            sets.push(format!("rcon_port = ${}", idx));
            idx += 1;
        }
        if update.rcon_password.is_some() {
            sets.push(format!("rcon_password = ${}", idx));
            idx += 1;
        }
        if update.volume_name.is_some() {
            sets.push(format!("volume_name = ${}", idx));
            idx += 1;
        }
        if update.last_error.is_some() {
            sets.push(format!("last_error = ${}", idx));
            // Pas d'incrementation : derniere binding dynamique. Si on ajoute
            // de nouveaux bindings dynamiques apres, restaurer le += 1.
            let _ = idx;
        } else if update.clear_last_error {
            sets.push("last_error = NULL".to_string());
        }
        if update.started_at_now {
            sets.push("started_at = NOW()".to_string());
        }
        if update.stopped_at_now {
            sets.push("stopped_at = NOW()".to_string());
        }
        sets.push("updated_at = NOW()".to_string());

        if sets.is_empty() {
            return Ok(());
        }

        let sql = format!(
            "UPDATE game_servers SET {} WHERE id = $1 AND deleted_at IS NULL",
            sets.join(", ")
        );
        let mut q = sqlx::query(&sql).bind(id);
        if let Some(ref s) = update.status {
            q = q.bind(s.as_str());
        }
        if let Some(ref v) = update.container_id {
            q = q.bind(v);
        }
        if let Some(ref v) = update.container_name {
            q = q.bind(v);
        }
        if let Some(p) = update.host_port {
            q = q.bind(p as i32);
        }
        if let Some(p) = update.rcon_port {
            q = q.bind(p as i32);
        }
        if let Some(ref v) = update.rcon_password {
            q = q.bind(v);
        }
        if let Some(ref v) = update.volume_name {
            q = q.bind(v);
        }
        if let Some(ref v) = update.last_error {
            q = q.bind(v);
        }

        q.execute(&self.pool)
            .await
            .map_err(pg_ctx("update_runtime"))?;
        Ok(())
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: GameServerStatus,
        last_error: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE game_servers \
             SET status = $2, last_error = $3, updated_at = NOW() \
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(status.as_str())
        .bind(last_error)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("update_status"))?;
        Ok(())
    }

    async fn try_transition_status(
        &self,
        id: Uuid,
        from: &[GameServerStatus],
        to: GameServerStatus,
    ) -> Result<bool, DomainError> {
        let from_strs: Vec<&str> = from.iter().map(|s| s.as_str()).collect();
        let res = sqlx::query(
            "UPDATE game_servers \
             SET status = $2, updated_at = NOW() \
             WHERE id = $1 AND status = ANY($3) AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(to.as_str())
        .bind(&from_strs)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("try_transition_status"))?;
        Ok(res.rows_affected() == 1)
    }

    async fn update_player_activity(&self, id: Uuid, player_count: i32) -> Result<(), DomainError> {
        // last_active_at est mis a jour seulement si player_count > 0.
        sqlx::query(
            "UPDATE game_servers SET \
                last_player_count = $2, \
                last_active_at = CASE WHEN $2 > 0 THEN NOW() ELSE last_active_at END, \
                updated_at = NOW() \
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(player_count)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("update_player_activity"))?;
        Ok(())
    }

    async fn record_restart_attempt(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE game_servers \
             SET restart_attempts = restart_attempts + 1, last_restart_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("record_restart_attempt"))?;
        Ok(())
    }

    async fn reset_restart_attempts(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE game_servers \
             SET restart_attempts = 0, updated_at = NOW() \
             WHERE id = $1 AND restart_attempts <> 0 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("reset_restart_attempts"))?;
        Ok(())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE game_servers \
             SET status = 'deleted', deleted_at = NOW(), updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("soft_delete game_server"))?;
        Ok(())
    }

    async fn count_active_for_guild(&self, guild_id: &str) -> Result<(i32, i32), DomainError> {
        let row: (i64, Option<i64>) = sqlx::query_as(
            "SELECT COUNT(*)::bigint, COALESCE(SUM(allocated_memory_mb), 0)::bigint \
             FROM game_servers \
             WHERE guild_id = $1 AND deleted_at IS NULL AND status != 'deleted'",
        )
        .bind(guild_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx("count_active"))?;
        let count = i32::try_from(row.0).unwrap_or(i32::MAX);
        let mem = i32::try_from(row.1.unwrap_or(0)).unwrap_or(i32::MAX);
        Ok((count, mem))
    }

    async fn template_usages(
        &self,
        template_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, TemplateUsage>, DomainError> {
        if template_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // active_count : serveurs non-deletes utilisant ce template.
        // last_activity : MAX(updated_at) sur TOUS les serveurs ayant utilise
        // le template (incluant deletes), pour respecter la grace period.
        let rows: Vec<(Uuid, i64, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
            "SELECT template_id, \
                    COUNT(*) FILTER (WHERE deleted_at IS NULL)::bigint, \
                    MAX(updated_at) \
             FROM game_servers \
             WHERE template_id = ANY($1) \
             GROUP BY template_id",
        )
        .bind(template_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("template_usages"))?;

        Ok(rows
            .into_iter()
            .map(|(template_id, active_count, last_activity_at)| {
                (
                    template_id,
                    TemplateUsage {
                        active_count: i32::try_from(active_count).unwrap_or(i32::MAX),
                        last_activity_at,
                    },
                )
            })
            .collect())
    }

    async fn set_session_channels(
        &self,
        id: Uuid,
        text_channel_id: Option<&str>,
        voice_channel_id: Option<&str>,
    ) -> Result<bool, DomainError> {
        // Claim garde (D) : quand on POSE des salons (valeur non nulle), on ne
        // le fait QUE si le serveur n'en a pas deja -> sur redelivrance de
        // l'event de demarrage, la 2e tentative echoue le claim (le bot pourra
        // supprimer ses salons dupliques). L'effacement (valeur nulle, a l'arret)
        // n'est pas garde.
        let res = sqlx::query(
            "UPDATE game_servers SET text_channel_id = $2, voice_channel_id = $3, updated_at = NOW() \
             WHERE id = $1 AND ($2 IS NULL OR text_channel_id IS NULL)",
        )
        .bind(id)
        .bind(text_channel_id)
        .bind(voice_channel_id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("set_session_channels"))?;
        Ok(res.rows_affected() == 1)
    }

    async fn mark_ip_revealed(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE game_servers SET ip_revealed = true, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("mark_ip_revealed"))?;
        Ok(())
    }

    async fn list_ip_reveal_due(&self) -> Result<Vec<GameServer>, DomainError> {
        let q = format!(
            "SELECT {SELECT_COLS} FROM game_servers \
             WHERE deleted_at IS NULL AND ip_revealed = false \
               AND ip_reveal_at IS NOT NULL AND ip_reveal_at <= NOW() \
               AND text_channel_id IS NOT NULL AND status = 'running' \
             ORDER BY ip_reveal_at ASC LIMIT 100"
        );
        let rows = sqlx::query_as::<_, ServerRow>(&q)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_ctx("list_ip_reveal_due"))?;
        rows.into_iter().map(GameServer::try_from).collect()
    }

    async fn list_awaiting_reveal_no_ping_today(&self) -> Result<Vec<GameServer>, DomainError> {
        let q = format!(
            "SELECT {SELECT_COLS} FROM game_servers \
             WHERE deleted_at IS NULL AND ip_revealed = false \
               AND ip_reveal_at IS NOT NULL AND ip_reveal_at > NOW() \
               AND text_channel_id IS NOT NULL \
               AND (last_daily_ping_at IS NULL OR last_daily_ping_at < date_trunc('day', NOW())) \
             ORDER BY ip_reveal_at ASC LIMIT 100"
        );
        let rows = sqlx::query_as::<_, ServerRow>(&q)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_ctx("list_awaiting_reveal_no_ping_today"))?;
        rows.into_iter().map(GameServer::try_from).collect()
    }

    async fn mark_daily_ping(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE game_servers SET last_daily_ping_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("mark_daily_ping"))?;
        Ok(())
    }

    async fn record_perf_sample(
        &self,
        id: Uuid,
        rcon_latency_ms: Option<i32>,
        net_rx_bytes: Option<i64>,
        net_tx_bytes: Option<i64>,
    ) -> Result<(), DomainError> {
        // `COALESCE` sur les compteurs : un echantillon reseau manquant (stats
        // Docker indisponibles) ne doit pas effacer le precedent, sinon le
        // debit repartirait de zero au coup d'apres.
        sqlx::query(
            "UPDATE game_servers SET                  rcon_latency_ms = $2,                  net_rx_bytes = COALESCE($3, net_rx_bytes),                  net_tx_bytes = COALESCE($4, net_tx_bytes),                  net_sampled_at = CASE WHEN $3 IS NULL THEN net_sampled_at ELSE NOW() END              WHERE id = $1",
        )
        .bind(id)
        .bind(rcon_latency_ms)
        .bind(net_rx_bytes)
        .bind(net_tx_bytes)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("record_perf_sample"))?;
        Ok(())
    }

    async fn set_config_dirty(&self, id: Uuid, dirty: bool) -> Result<(), DomainError> {
        sqlx::query("UPDATE game_servers SET config_dirty = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(dirty)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("set_config_dirty"))?;
        Ok(())
    }

    async fn set_closes_at(
        &self,
        id: Uuid,
        at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE game_servers SET closes_at = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(at)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("set_closes_at"))?;
        Ok(())
    }

    async fn set_ip_reveal_at(
        &self,
        id: Uuid,
        at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE game_servers SET ip_reveal_at = $2, ip_revealed = false, updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(id)
        .bind(at)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("set_ip_reveal_at"))?;
        Ok(())
    }

    async fn list_scheduled_due_to_start(&self) -> Result<Vec<GameServer>, DomainError> {
        // Fenetre de 5 minutes = PREP_LEAD_MINUTES (domaine). L'intervalle est
        // ecrit en dur ici faute de pouvoir binder un INTERVAL Postgres proprement ;
        // garder les deux valeurs synchronisees.
        let q = format!(
            "SELECT {SELECT_COLS} FROM game_servers \
             WHERE deleted_at IS NULL AND status = 'scheduled' \
               AND ip_reveal_at IS NOT NULL \
               AND ip_reveal_at <= NOW() + INTERVAL '5 minutes' \
             ORDER BY ip_reveal_at ASC LIMIT 100"
        );
        let rows = sqlx::query_as::<_, ServerRow>(&q)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_ctx("list_scheduled_due_to_start"))?;
        rows.into_iter().map(GameServer::try_from).collect()
    }
}
