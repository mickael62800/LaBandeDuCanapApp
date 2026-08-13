//! Implementation Redis du port allocator.
//!
//! Strategie : pour chaque port du range, une cle Redis `game:port:{kind}:{port}`
//! contenant l'owner_key (server_id). Allocation = SETNX avec TTL long
//! (7 jours par defaut, refresh sur usage). Liberation = DEL.

use async_trait::async_trait;
use rand::Rng;
use redis::AsyncCommands;
use redis::Client;

use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::port_allocator::{PortAllocator, PortKind};

/// TTL PAR DEFAUT d'une reservation de port (secondes = 7 jours). Global /
/// infra : la reservation est un simple garde-fou anti-collision au niveau
/// runtime, sans semantique per-guild. Surchargeable via l'env
/// `GAME_PORTAL_PORT_RESERVATION_TTL_SECS`.
const DEFAULT_KEY_TTL_SECS: u64 = 60 * 60 * 24 * 7; // 7j (refresh sur usage)

pub struct RedisPortAllocator {
    client: Client,
    ttl_secs: u64,
}

impl RedisPortAllocator {
    pub fn new(client: Client) -> Self {
        let ttl_secs = std::env::var("GAME_PORTAL_PORT_RESERVATION_TTL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|&v| v > 0)
            .unwrap_or(DEFAULT_KEY_TTL_SECS);
        Self { client, ttl_secs }
    }

    fn key(kind: PortKind, port: u16) -> String {
        let prefix = match kind {
            PortKind::Game => "game",
            PortKind::Rcon => "rcon",
        };
        format!("game:port:{prefix}:{port}")
    }

    /// Parcourt toute la plage une seule fois, depuis une position aleatoire.
    /// On conserve ainsi la garantie de trouver un slot libre sans attribuer
    /// systematiquement les ports dans l'ordre croissant.
    fn randomized_candidates(range_start: u16, range_end: u16) -> Vec<u16> {
        let count = u32::from(range_end) - u32::from(range_start) + 1;
        let offset = rand::thread_rng().gen_range(0..count);
        Self::rotated_candidates(range_start, range_end, offset)
    }

    fn rotated_candidates(range_start: u16, range_end: u16, offset: u32) -> Vec<u16> {
        let count = u32::from(range_end) - u32::from(range_start) + 1;
        (0..count)
            .map(|index| u32::from(range_start) + (offset + index) % count)
            .map(|port| port as u16)
            .collect()
    }
}

#[async_trait]
impl PortAllocator for RedisPortAllocator {
    async fn allocate(
        &self,
        kind: PortKind,
        range_start: u16,
        range_end: u16,
        owner_key: &str,
    ) -> Result<u16, DomainError> {
        if range_start > range_end {
            return Err(DomainError::ValidationError(
                "range_start > range_end".into(),
            ));
        }
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DomainError::Internal(format!("redis conn: {e}")))?;

        for port in Self::randomized_candidates(range_start, range_end) {
            let key = Self::key(kind, port);
            // SET NX EX (atomic). Retourne true si on a gagne le slot.
            let won: bool = redis::cmd("SET")
                .arg(&key)
                .arg(owner_key)
                .arg("NX")
                .arg("EX")
                .arg(self.ttl_secs)
                .query_async(&mut conn)
                .await
                .map_err(|e| DomainError::Internal(format!("redis SET NX: {e}")))?;
            if won {
                return Ok(port);
            }
        }
        Err(DomainError::ValidationError(
            "aucun port libre dans le range configure".into(),
        ))
    }

    async fn allocate_block(
        &self,
        kind: PortKind,
        range_start: u16,
        range_end: u16,
        width: u16,
        owner_key: &str,
    ) -> Result<u16, DomainError> {
        if width == 0 || range_end.saturating_sub(range_start) + 1 < width {
            return Err(DomainError::ValidationError(
                "plage insuffisante pour le bloc de ports".into(),
            ));
        }
        let last_start = range_end - (width - 1);
        let candidates = Self::randomized_candidates(range_start, last_start);
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DomainError::Internal(format!("redis conn: {e}")))?;
        // Verifie puis reserve toutes les cles dans le MEME script Redis :
        // aucun autre create ne peut prendre +1/+2 entre les deux etapes.
        let script = redis::Script::new(
            "for i=1,#KEYS do if redis.call('EXISTS', KEYS[i]) == 1 then return 0 end end \
             for i=1,#KEYS do redis.call('SET', KEYS[i], ARGV[1], 'EX', ARGV[2]) end return 1",
        );
        for start in candidates {
            let mut invocation = script.prepare_invoke();
            for port in start..start + width {
                invocation.key(Self::key(kind, port));
            }
            let won: i32 = invocation
                .arg(owner_key)
                .arg(self.ttl_secs)
                .invoke_async(&mut conn)
                .await
                .map_err(|e| DomainError::Internal(format!("redis reserve block: {e}")))?;
            if won == 1 {
                return Ok(start);
            }
        }
        Err(DomainError::ValidationError(
            "aucun bloc de ports libre dans le range configure".into(),
        ))
    }

    async fn release(&self, kind: PortKind, port: u16) -> Result<(), DomainError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DomainError::Internal(format!("redis conn: {e}")))?;
        let _: i64 = conn
            .del(Self::key(kind, port))
            .await
            .map_err(|e| DomainError::Internal(format!("redis DEL: {e}")))?;
        Ok(())
    }

    async fn is_available(&self, kind: PortKind, port: u16) -> Result<bool, DomainError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DomainError::Internal(format!("redis conn: {e}")))?;
        let exists: bool = conn
            .exists(Self::key(kind, port))
            .await
            .map_err(|e| DomainError::Internal(format!("redis EXISTS: {e}")))?;
        Ok(!exists)
    }
}

#[cfg(test)]
mod tests {
    use super::RedisPortAllocator;

    #[test]
    fn rotated_candidates_wrap_and_cover_the_whole_range() {
        assert_eq!(
            RedisPortAllocator::rotated_candidates(25500, 25504, 3),
            vec![25503, 25504, 25500, 25501, 25502]
        );
    }

    #[test]
    fn single_port_range_is_supported() {
        assert_eq!(
            RedisPortAllocator::rotated_candidates(25500, 25500, 0),
            vec![25500]
        );
    }
}
