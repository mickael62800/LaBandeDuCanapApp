use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;

/// Port outbound : comptage des events d'audit par `event_type` sur une fenetre
/// glissante (en jours), pour un guild donne.
///
/// C'est la lecture agregee de la table `audit_logs` qui alimente le rapport
/// hebdomadaire. L'implementation postgres fait un `GROUP BY event_type` avec un
/// filtre `created_at > now() - interval`.
#[async_trait]
pub trait AuditEventCounter: Send + Sync {
    /// Retourne, pour `guild_id`, les couples `(event_type, count)` des events
    /// survenus dans les `days` derniers jours.
    async fn count_by_event_type(
        &self,
        guild_id: &str,
        days: u32,
    ) -> Result<Vec<(String, u64)>, DomainError>;
}
