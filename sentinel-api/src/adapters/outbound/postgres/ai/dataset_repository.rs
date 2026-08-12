//! Adapter sortant Postgres du dataset IA (`ai_dataset_messages`). Tout le SQL
//! du domaine dataset vit ici : listing filtre + suppression en masse.

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::adapters::outbound::postgres::pg_err;
use sentinel_core::domain::entities::ai::dataset::{DatasetMessage, DatasetPage, DatasetQuery};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::ai::dataset_repository::{
    DatasetRepository, NewDatasetMessage,
};

pub struct PgDatasetRepository {
    pool: PgPool,
}

impl PgDatasetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DatasetRepository for PgDatasetRepository {
    async fn insert_message(&self, msg: &NewDatasetMessage) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO ai_dataset_messages (guild_id, channel_id, channel_name, user_id, content) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&msg.guild_id)
        .bind(&msg.channel_id)
        .bind(msg.channel_name.as_deref())
        .bind(&msg.user_id)
        .bind(&msg.content)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_messages(&self, query: &DatasetQuery) -> Result<DatasetPage, DomainError> {
        // Construction dynamique securisee (params bindes via $N).
        let mut sql = String::from(
            "SELECT id::text, user_id, channel_id, channel_name, content, \
                    to_char(created_at, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') \
             FROM ai_dataset_messages \
             WHERE guild_id = $1 \
               AND length(content) >= $2",
        );
        let mut count_sql = String::from(
            "SELECT COUNT(*)::bigint FROM ai_dataset_messages \
             WHERE guild_id = $1 \
               AND length(content) >= $2",
        );
        let mut idx = 3;
        if query.channel_id.is_some() {
            let f = format!(" AND channel_id = ${idx}");
            sql.push_str(&f);
            count_sql.push_str(&f);
            idx += 1;
        }
        if query.from.is_some() {
            let f = format!(" AND created_at >= ${idx}::timestamptz");
            sql.push_str(&f);
            count_sql.push_str(&f);
            idx += 1;
        }
        if query.to.is_some() {
            let f = format!(" AND created_at <= ${idx}::timestamptz");
            sql.push_str(&f);
            count_sql.push_str(&f);
            idx += 1;
        }
        sql.push_str(&format!(
            " ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            idx,
            idx + 1
        ));

        let mut q_items = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<String>,
                Option<String>,
                String,
                String,
            ),
        >(&sql)
        .bind(&query.guild_id)
        .bind(query.min_length);
        let mut q_count = sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(&query.guild_id)
            .bind(query.min_length);
        if let Some(c) = &query.channel_id {
            q_items = q_items.bind(c);
            q_count = q_count.bind(c);
        }
        if let Some(f) = &query.from {
            q_items = q_items.bind(f);
            q_count = q_count.bind(f);
        }
        if let Some(t) = &query.to {
            q_items = q_items.bind(t);
            q_count = q_count.bind(t);
        }
        q_items = q_items.bind(query.limit).bind(query.offset);

        let rows = q_items.fetch_all(&self.pool).await.map_err(pg_err)?;
        // Le COUNT remonte son erreur comme le SELECT. En `unwrap_or(0)`, une
        // base en panne rendait un `total: 0` indiscernable d'un dataset vide :
        // la pagination disparaissait de l'ecran sans qu'aucune erreur ne soit
        // affichee, et le probleme se lisait « il n'y a plus de messages ».
        let total = q_count.fetch_one(&self.pool).await.map_err(pg_err)?;

        let items = rows
            .into_iter()
            .map(
                |(id, user_id, channel_id, channel_name, content, created_at)| DatasetMessage {
                    id,
                    user_id,
                    channel_id,
                    channel_name,
                    content,
                    created_at,
                },
            )
            .collect();

        Ok(DatasetPage { items, total })
    }

    async fn bulk_delete(&self, guild_id: &str, ids: &[Uuid]) -> Result<i64, DomainError> {
        let res = sqlx::query(
            "DELETE FROM ai_dataset_messages \
             WHERE guild_id = $1 \
               AND id = ANY($2)",
        )
        .bind(guild_id)
        .bind(ids)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(res.rows_affected() as i64)
    }
}
