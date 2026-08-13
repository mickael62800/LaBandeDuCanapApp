use async_trait::async_trait;
use sqlx::PgPool;

use super::super::pg_err;
use platform_core::sentinel::ports::outbound::moderation::evidence_repository::EvidenceEntry;
use platform_core::sentinel::ports::outbound::moderation::evidence_repository::EvidenceRepository;

pub struct PgEvidenceRepository {
    pool: PgPool,
}

impl PgEvidenceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: uuid::Uuid,
    url: String,
    description: Option<String>,
    uploaded_by: String,
    uploaded_by_name: String,
    uploaded_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl EvidenceRepository for PgEvidenceRepository {
    async fn add(
        &self,
        action_id: uuid::Uuid,
        url: &str,
        description: Option<&str>,
        uploaded_by: &str,
        uploaded_by_name: &str,
    ) -> Result<EvidenceEntry, platform_core::sentinel::domain::errors::DomainError> {
        let row: Row = sqlx::query_as(
            "INSERT INTO moderation_evidence \
                 (action_id, action_created_at, url, description, uploaded_by, uploaded_by_name) \
             SELECT a.id, a.created_at, $2, $3, $4, $5 \
             FROM audit_logs a \
             WHERE a.id = $1 AND a.event_type LIKE 'mod_%' \
             ORDER BY a.created_at DESC LIMIT 1 \
             RETURNING id, url, description, uploaded_by, uploaded_by_name, uploaded_at",
        )
        .bind(action_id)
        .bind(url)
        .bind(description)
        .bind(uploaded_by)
        .bind(uploaded_by_name)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(EvidenceEntry {
            id: row.id,
            action_id,
            url: row.url,
            description: row.description,
            uploaded_by: row.uploaded_by,
            uploaded_by_name: row.uploaded_by_name,
            uploaded_at: row.uploaded_at,
        })
    }

    async fn list(
        &self,
        action_id: uuid::Uuid,
    ) -> Result<Vec<EvidenceEntry>, platform_core::sentinel::domain::errors::DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, url, description, uploaded_by, uploaded_by_name, uploaded_at \
             FROM moderation_evidence WHERE action_id = $1 ORDER BY uploaded_at ASC",
        )
        .bind(action_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| EvidenceEntry {
                id: r.id,
                action_id,
                url: r.url,
                description: r.description,
                uploaded_by: r.uploaded_by,
                uploaded_by_name: r.uploaded_by_name,
                uploaded_at: r.uploaded_at,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "../sentinel-api/migrations")]
    async fn evidence_is_linked_to_partitioned_audit_action(pool: PgPool) -> sqlx::Result<()> {
        let action_id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO audit_logs (id, guild_id, event_type, target_id, details) \
             VALUES ($1, 'guild-1', 'mod_warn', 'target-1', '{}'::jsonb)",
        )
        .bind(action_id)
        .execute(&pool)
        .await?;

        let repo = PgEvidenceRepository::new(pool);
        repo.add(
            action_id,
            "https://example.test/evidence",
            Some("capture"),
            "admin-1",
            "Admin",
        )
        .await
        .unwrap();
        let evidence = repo.list(action_id).await.unwrap();

        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].action_id, action_id);
        assert_eq!(evidence[0].description.as_deref(), Some("capture"));
        Ok(())
    }
}
