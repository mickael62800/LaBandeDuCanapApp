use async_trait::async_trait;
use redis::AsyncCommands;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::rule::Rule;
use platform_core::sentinel::domain::enums::moderation::flag_type::FlagType;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::system::cache::CachePort;

const RULES_TTL: u64 = 300; // 5 minutes

pub struct RedisCache {
    /// Connexion multiplexee persistante (cloneable, partage le meme socket TCP).
    conn: redis::aio::MultiplexedConnection,
    /// Compteur de cache hits.
    hits: AtomicU64,
    /// Compteur de cache misses.
    misses: AtomicU64,
}

/// Statistiques du cache Redis.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total: u64,
    pub hit_rate_percent: f64,
}

impl RedisCache {
    pub async fn new(client: redis::Client) -> Result<Self, redis::RedisError> {
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self {
            conn,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        })
    }

    fn rules_key(guild_id: &str) -> String {
        format!("rules:{guild_id}")
    }

    async fn conn(&self) -> Result<redis::aio::MultiplexedConnection, DomainError> {
        Ok(self.conn.clone())
    }

    fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Retourne les statistiques du cache (hits, misses, hit rate).
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        CacheStats {
            hits,
            misses,
            total,
            hit_rate_percent: (hit_rate * 10.0).round() / 10.0,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedRule {
    id: String,
    guild_id: GuildId,
    flag_type: String,
    weight: f64,
    threshold_warn: f64,
    threshold_delete: f64,
    threshold_mute: f64,
    threshold_ban: f64,
    enabled: bool,
}

impl From<&Rule> for CachedRule {
    fn from(r: &Rule) -> Self {
        Self {
            id: r.id.to_string(),
            guild_id: r.guild_id.clone(),
            flag_type: r.flag_type.as_str().to_string(),
            weight: r.weight,
            threshold_warn: r.threshold_warn,
            threshold_delete: r.threshold_delete,
            threshold_mute: r.threshold_mute,
            threshold_ban: r.threshold_ban,
            enabled: r.enabled,
        }
    }
}

impl CachedRule {
    fn into_rule(self) -> Rule {
        Rule {
            id: self.id.parse().unwrap_or_else(|_| {
                tracing::warn!("Invalid UUID in cache: {}, using nil", self.id);
                uuid::Uuid::nil()
            }),
            guild_id: self.guild_id,
            flag_type: FlagType::from_str_lossy(&self.flag_type),
            weight: self.weight,
            threshold_warn: self.threshold_warn,
            threshold_delete: self.threshold_delete,
            threshold_mute: self.threshold_mute,
            threshold_ban: self.threshold_ban,
            enabled: self.enabled,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}

#[async_trait]
impl CachePort for RedisCache {
    // --- Rules cache ---

    async fn get_rules(&self, guild_id: &str) -> Result<Option<Vec<Rule>>, DomainError> {
        let mut conn = self.conn().await?;

        let data: Option<String> = conn
            .get(Self::rules_key(guild_id))
            .await
            .map_err(|e| DomainError::Internal(format!("Redis GET: {e}")))?;

        match data {
            Some(json) => {
                self.record_hit();
                let cached: Vec<CachedRule> = serde_json::from_str(&json)
                    .map_err(|e| DomainError::Internal(format!("Redis deserialize: {e}")))?;
                Ok(Some(cached.into_iter().map(|c| c.into_rule()).collect()))
            }
            None => {
                self.record_miss();
                Ok(None)
            }
        }
    }

    async fn set_rules(&self, guild_id: &str, rules: &[Rule]) -> Result<(), DomainError> {
        let mut conn = self.conn().await?;

        let cached: Vec<CachedRule> = rules.iter().map(CachedRule::from).collect();
        let json =
            serde_json::to_string(&cached).map_err(|e| DomainError::Internal(e.to_string()))?;

        conn.set_ex::<_, _, ()>(Self::rules_key(guild_id), json, RULES_TTL)
            .await
            .map_err(|e| DomainError::Internal(format!("Redis SETEX: {e}")))?;

        Ok(())
    }

    async fn invalidate_rules(&self, guild_id: &str) -> Result<(), DomainError> {
        let mut conn = self.conn().await?;

        conn.del::<_, ()>(Self::rules_key(guild_id))
            .await
            .map_err(|e| DomainError::Internal(format!("Redis DEL: {e}")))?;

        Ok(())
    }

    // --- Generic JSON cache ---

    async fn get_json(&self, key: &str) -> Result<Option<String>, DomainError> {
        let mut conn = self.conn().await?;

        let result: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| DomainError::Internal(format!("Redis GET {key}: {e}")))?;

        match &result {
            Some(_) => self.record_hit(),
            None => self.record_miss(),
        }

        Ok(result)
    }

    async fn set_json(&self, key: &str, json: &str, ttl_secs: u64) -> Result<(), DomainError> {
        let mut conn = self.conn().await?;

        conn.set_ex::<_, _, ()>(key, json, ttl_secs)
            .await
            .map_err(|e| DomainError::Internal(format!("Redis SETEX {key}: {e}")))?;

        Ok(())
    }

    async fn invalidate(&self, key: &str) -> Result<(), DomainError> {
        let mut conn = self.conn().await?;

        conn.del::<_, ()>(key)
            .await
            .map_err(|e| DomainError::Internal(format!("Redis DEL {key}: {e}")))?;

        Ok(())
    }

    async fn invalidate_pattern(&self, pattern: &str) -> Result<(), DomainError> {
        let mut conn = self.conn().await?;

        // Utilise SCAN au lieu de KEYS pour ne pas bloquer Redis
        let mut cursor = 0u64;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| DomainError::Internal(format!("Redis SCAN {pattern}: {e}")))?;

            for key in &keys {
                if let Err(e) = conn.del::<_, ()>(key).await {
                    tracing::warn!(error = %e, key = %key, "Echec Redis DEL dans invalidate_pattern");
                }
            }

            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/redis_cache.rs"]
mod tests;
