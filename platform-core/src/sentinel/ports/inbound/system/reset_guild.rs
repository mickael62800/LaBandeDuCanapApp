use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::outbound::system::guild_reset_repository::ResetDiscordContext;

/// Resultat d'un reset complet d'un serveur.
#[derive(Debug, Clone)]
pub struct ResetGuildOutcome {
    /// Contexte Discord a transmettre au bot (roles a retirer, etc.).
    pub discord_context: ResetDiscordContext,
    /// Detail des suppressions `(table, lignes)`.
    pub tables_wiped: Vec<(String, u64)>,
    /// Total de lignes supprimees.
    pub total_rows: u64,
}

#[async_trait]
pub trait ResetGuildUseCase: Send + Sync {
    /// Efface toutes les donnees du serveur (IRREVERSIBLE). Le controle owner
    /// est assure par l'adapter HTTP (RBAC). `confirmation` doit etre EXACTEMENT
    /// le nom du serveur (garde-fou anti-clic accidentel), verifie ici.
    /// Renvoie `Forbidden` si la confirmation ne correspond pas, `NotFound` si
    /// le serveur est inconnu.
    async fn reset(
        &self,
        guild_id: &str,
        confirmation: &str,
    ) -> Result<ResetGuildOutcome, DomainError>;
}
