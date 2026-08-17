//! Port outbound : conservation du dernier inventaire Discord d'une guilde.
//!
//! Le bot est le seul composant qui voit Discord ; l'API est la seule a voir la
//! base. Aucun des deux ne peut donc comparer seul. Le bot depose ici sa
//! photographie de la guilde, et le rapport de divergence se calcule a la
//! demande, en la confrontant a l'etat enregistre.
//!
//! Un inventaire est jetable : seul le dernier compte, et son absence signifie
//! « on ne sait pas », jamais « tout va bien ».

use async_trait::async_trait;

use crate::nexus::domain::entities::casino::game_sync::DiscordInventory;
use crate::nexus::domain::errors::DomainError;

/// Inventaire accompagne de sa date de prise (RFC3339). Un inventaire vieux de
/// plusieurs heures reste exploitable, mais l'ecran doit pouvoir dire son age :
/// resoudre un ecart sur une photo perimee ferait defaire un travail deja fait.
#[derive(Debug, Clone)]
pub struct StoredInventory {
    pub inventory: DiscordInventory,
    pub taken_at: String,
}

#[async_trait]
pub trait GameSyncRepository: Send + Sync {
    /// Remplace l'inventaire de la guilde. Le precedent n'a plus d'interet.
    async fn save_inventory(
        &self,
        guild_id: &str,
        inventory: &DiscordInventory,
    ) -> Result<(), DomainError>;

    /// Dernier inventaire connu, ou `None` si le bot n'a jamais rendu compte.
    async fn latest_inventory(
        &self,
        guild_id: &str,
    ) -> Result<Option<StoredInventory>, DomainError>;

    /// Guildes ayant au moins un jeu mentionnable, pour la verification
    /// periodique. Sans cela le job devrait deviner ou chercher.
    async fn guilds_with_games(&self) -> Result<Vec<String>, DomainError>;
}
