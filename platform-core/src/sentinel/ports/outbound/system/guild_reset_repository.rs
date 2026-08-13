use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;

/// Contexte Discord collecte AVANT le wipe (les ids necessaires au bot pour
/// annuler l'etat Discord, car ces donnees seront effacees de la base).
#[derive(Debug, Clone, Default)]
pub struct ResetDiscordContext {
    /// Role de quarantaine configure (a retirer des membres).
    pub quarantine_role_id: Option<String>,
    /// Ids des roles temporaires poses par le bot (a retirer).
    pub temp_role_ids: Vec<String>,
}

/// Repository du "factory reset" par serveur : efface toutes les donnees d'un
/// guild. Operation IRREVERSIBLE — reservee a l'owner avec confirmation forte.
#[async_trait]
pub trait GuildResetRepository: Send + Sync {
    /// Nom du serveur (pour la confirmation forte). `None` si inconnu.
    async fn guild_name(&self, guild_id: &str) -> Result<Option<String>, DomainError>;

    /// Collecte les ids Discord necessaires au bot AVANT le wipe.
    async fn collect_discord_context(
        &self,
        guild_id: &str,
    ) -> Result<ResetDiscordContext, DomainError>;

    /// Efface toutes les lignes `WHERE guild_id = $1` de toutes les tables
    /// guild-scopees (sauf exclusions : `guilds`, RBAC, `bot_definitions`),
    /// dans une transaction. Retourne `(table, lignes_supprimees)`.
    async fn wipe_guild(&self, guild_id: &str) -> Result<Vec<(String, u64)>, DomainError>;
}
