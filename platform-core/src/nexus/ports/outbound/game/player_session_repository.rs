use async_trait::async_trait;
use uuid::Uuid;

use crate::nexus::domain::entities::game::player_session::PlayerSession;
use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait PlayerSessionRepository: Send + Sync {
    /// Cree une nouvelle session (joueur vient de se connecter).
    async fn open(&self, server_id: Uuid, player_name: &str) -> Result<Uuid, DomainError>;

    /// Cloture une session active (set left_at = NOW()).
    /// Si plusieurs sessions actives pour ce joueur, ferme la plus ancienne.
    async fn close(&self, server_id: Uuid, player_name: &str) -> Result<(), DomainError>;

    /// Liste les sessions actuellement actives d'un serveur (pour diff).
    async fn list_active(&self, server_id: Uuid) -> Result<Vec<PlayerSession>, DomainError>;

    /// Histoire paginee des sessions d'un serveur.
    async fn list_history(
        &self,
        server_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PlayerSession>, DomainError>;

    /// Nombre total de sessions d'un serveur, toutes pages confondues.
    ///
    /// Sans lui, une liste paginee ne peut annoncer ni son nombre de pages ni
    /// son total : elle affiche une page en laissant croire qu'il n'y a rien
    /// derriere. Compter est une requete separee parce que `LIMIT` masque
    /// justement ce qu'on cherche a savoir.
    async fn count_history(&self, server_id: Uuid) -> Result<i64, DomainError>;

    /// Force-cloture toutes les sessions actives (utilise au stop / crash).
    async fn close_all_active(&self, server_id: Uuid) -> Result<(), DomainError>;
}
