//! Quotas et limites systeme — calculs purs (pas d'I/O).

use serde::{Deserialize, Serialize};

/// Etat actuel des quotas pour une guild (calcule a partir des serveurs
/// existants + lu depuis bot_config game-portal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildQuotaState {
    /// Nb de serveurs actifs (created/starting/running/stopping/stopped — non deleted).
    pub active_servers: i32,
    pub max_servers: i32,
    /// Memoire allouee cumulee (Mo) tous serveurs actifs.
    pub allocated_memory_mb: i32,
    pub max_memory_mb: i32,
}

impl GuildQuotaState {
    pub fn can_create_server(&self, requested_memory_mb: i32) -> Result<(), QuotaError> {
        if self.active_servers >= self.max_servers {
            return Err(QuotaError::TooManyServers {
                current: self.active_servers,
                max: self.max_servers,
            });
        }
        if self.allocated_memory_mb + requested_memory_mb > self.max_memory_mb {
            return Err(QuotaError::MemoryExceeded {
                requested: requested_memory_mb,
                already_allocated: self.allocated_memory_mb,
                max: self.max_memory_mb,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum QuotaError {
    #[error("quota serveurs atteint: {current}/{max}")]
    TooManyServers { current: i32, max: i32 },
    #[error("memoire totale depasseee: {requested} Mo demandes + {already_allocated} Mo deja alloues > {max} Mo plafond")]
    MemoryExceeded {
        requested: i32,
        already_allocated: i32,
        max: i32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quota_create_ok_when_within_limits() {
        let q = GuildQuotaState {
            active_servers: 2,
            max_servers: 5,
            allocated_memory_mb: 2048,
            max_memory_mb: 8192,
        };
        assert!(q.can_create_server(2048).is_ok());
    }

    #[test]
    fn quota_blocks_too_many_servers() {
        let q = GuildQuotaState {
            active_servers: 5,
            max_servers: 5,
            allocated_memory_mb: 0,
            max_memory_mb: 8192,
        };
        let err = q.can_create_server(1024).unwrap_err();
        matches!(err, QuotaError::TooManyServers { .. });
    }

    #[test]
    fn quota_blocks_memory_overflow() {
        let q = GuildQuotaState {
            active_servers: 1,
            max_servers: 5,
            allocated_memory_mb: 7000,
            max_memory_mb: 8192,
        };
        let err = q.can_create_server(2048).unwrap_err();
        matches!(err, QuotaError::MemoryExceeded { .. });
    }
}
