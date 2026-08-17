use async_trait::async_trait;
use sqlx::PgPool;

use super::super::pg_err;
use platform_core::nexus::domain::entities::casino::game_sync::DiscordInventory;
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::casino::game_sync_repository::{
    GameSyncRepository, StoredInventory,
};

pub struct PgGameSyncRepository {
    pool: PgPool,
}

impl PgGameSyncRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GameSyncRepository for PgGameSyncRepository {
    async fn save_inventory(
        &self,
        guild_id: &str,
        inventory: &DiscordInventory,
    ) -> Result<(), DomainError> {
        let payload = serde_json::to_value(inventory)
            .map_err(|e| DomainError::Internal(format!("inventaire illisible: {e}")))?;

        // Une seule photographie par guilde : la precedente n'a plus d'interet,
        // et en garder l'historique ferait grossir la table sans usage.
        sqlx::query(
            "INSERT INTO game_sync_inventory (guild_id, inventory, taken_at) \
             VALUES ($1, $2, NOW()) \
             ON CONFLICT (guild_id) DO UPDATE SET \
               inventory = EXCLUDED.inventory, \
               taken_at = EXCLUDED.taken_at",
        )
        .bind(guild_id)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn latest_inventory(
        &self,
        guild_id: &str,
    ) -> Result<Option<StoredInventory>, DomainError> {
        let row: Option<(serde_json::Value, String)> = sqlx::query_as(
            "SELECT inventory, to_char(taken_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') \
             FROM game_sync_inventory WHERE guild_id = $1",
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        let Some((payload, taken_at)) = row else {
            return Ok(None);
        };

        // Un inventaire illisible vaut une absence d'inventaire : le domaine
        // n'affirmera alors aucun ecart, plutot que d'en deduire de faux.
        match serde_json::from_value::<DiscordInventory>(payload) {
            Ok(inventory) => Ok(Some(StoredInventory {
                inventory,
                taken_at,
            })),
            Err(error) => {
                tracing::warn!(%error, guild_id, "inventaire de synchronisation illisible, ignore");
                Ok(None)
            }
        }
    }

    async fn guilds_with_games(&self) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT DISTINCT guild_id FROM games")
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(|(guild_id,)| guild_id).collect())
    }
}
