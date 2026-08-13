use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::embed::{Embed, EmbedField};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::embed_repository::EmbedRepository;

pub struct PgEmbedRepository {
    pool: PgPool,
}

impl PgEmbedRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct EmbedRow {
    id: Uuid,
    guild_id: String,
    name: String,
    content: String,
    author_name: String,
    author_icon_url: String,
    author_url: String,
    title: String,
    title_url: String,
    description: String,
    color: Option<i32>,
    image_url: String,
    thumbnail_url: String,
    footer_text: String,
    footer_icon_url: String,
    show_timestamp: bool,
    fields: serde_json::Value,
    last_channel_id: Option<String>,
    last_message_id: Option<String>,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<EmbedRow> for Embed {
    fn from(r: EmbedRow) -> Self {
        let fields: Vec<EmbedField> = serde_json::from_value(r.fields).unwrap_or_default();
        Self {
            id: r.id,
            guild_id: r.guild_id,
            name: r.name,
            content: r.content,
            author_name: r.author_name,
            author_icon_url: r.author_icon_url,
            author_url: r.author_url,
            title: r.title,
            title_url: r.title_url,
            description: r.description,
            color: r.color,
            image_url: r.image_url,
            thumbnail_url: r.thumbnail_url,
            footer_text: r.footer_text,
            footer_icon_url: r.footer_icon_url,
            show_timestamp: r.show_timestamp,
            fields,
            last_channel_id: r.last_channel_id,
            last_message_id: r.last_message_id,
            created_by: r.created_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const SELECT_EMBED: &str = r#"
    SELECT id, guild_id, name, content,
        author_name, author_icon_url, author_url,
        title, title_url, description, color,
        image_url, thumbnail_url,
        footer_text, footer_icon_url, show_timestamp, fields,
        last_channel_id, last_message_id,
        created_by, created_at, updated_at
    FROM embeds
"#;

#[async_trait]
impl EmbedRepository for PgEmbedRepository {
    async fn create(&self, e: &Embed) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO embeds (
                id, guild_id, name, content,
                author_name, author_icon_url, author_url,
                title, title_url, description, color,
                image_url, thumbnail_url,
                footer_text, footer_icon_url, show_timestamp, fields,
                created_by, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                $13, $14, $15, $16, $17, $18, $19, $20
            )"#,
        )
        .bind(e.id)
        .bind(&e.guild_id)
        .bind(&e.name)
        .bind(&e.content)
        .bind(&e.author_name)
        .bind(&e.author_icon_url)
        .bind(&e.author_url)
        .bind(&e.title)
        .bind(&e.title_url)
        .bind(&e.description)
        .bind(e.color)
        .bind(&e.image_url)
        .bind(&e.thumbnail_url)
        .bind(&e.footer_text)
        .bind(&e.footer_icon_url)
        .bind(e.show_timestamp)
        .bind(serde_json::to_value(&e.fields).unwrap_or_default())
        .bind(&e.created_by)
        .bind(e.created_at)
        .bind(e.updated_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn update(&self, e: &Embed) -> Result<(), DomainError> {
        sqlx::query(
            r#"UPDATE embeds SET
                name = $2, content = $3,
                author_name = $4, author_icon_url = $5, author_url = $6,
                title = $7, title_url = $8, description = $9, color = $10,
                image_url = $11, thumbnail_url = $12,
                footer_text = $13, footer_icon_url = $14, show_timestamp = $15,
                fields = $16, updated_at = NOW()
            WHERE id = $1"#,
        )
        .bind(e.id)
        .bind(&e.name)
        .bind(&e.content)
        .bind(&e.author_name)
        .bind(&e.author_icon_url)
        .bind(&e.author_url)
        .bind(&e.title)
        .bind(&e.title_url)
        .bind(&e.description)
        .bind(e.color)
        .bind(&e.image_url)
        .bind(&e.thumbnail_url)
        .bind(&e.footer_text)
        .bind(&e.footer_icon_url)
        .bind(e.show_timestamp)
        .bind(serde_json::to_value(&e.fields).unwrap_or_default())
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM embeds WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Embed>, DomainError> {
        let q = format!("{SELECT_EMBED} WHERE id = $1");
        let row = sqlx::query_as::<_, EmbedRow>(&q)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Embed::from))
    }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Embed>, DomainError> {
        let q = format!("{SELECT_EMBED} WHERE guild_id = $1 ORDER BY updated_at DESC");
        let rows = sqlx::query_as::<_, EmbedRow>(&q)
            .bind(guild_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Embed::from).collect())
    }

    async fn set_last_post(
        &self,
        id: Uuid,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE embeds SET last_channel_id = $2, last_message_id = $3, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(channel_id)
        .bind(message_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
