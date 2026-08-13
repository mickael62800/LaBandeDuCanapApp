use super::*;

impl PgAutomodReviewRepository {
    pub(super) async fn find_discussion_impl(
        &self,
        review_id: Uuid,
    ) -> Result<Option<DiscussionChannel>, DomainError> {
        let row: Option<DiscussionRow> =
            sqlx::query_as("SELECT * FROM automod_discussion_channels WHERE review_id = $1")
                .bind(review_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    pub(super) async fn create_discussion_impl(
        &self,
        d: NewDiscussionChannel,
    ) -> Result<(DiscussionChannel, bool), DomainError> {
        // Idempotence : UNIQUE(review_id). On tente l'insert ; en cas de
        // conflit on renvoie l'existant avec created=false.
        let inserted: Option<DiscussionRow> = sqlx::query_as(
            "INSERT INTO automod_discussion_channels \
                (review_id, guild_id, channel_id, opened_by_id, opened_by_name) \
             VALUES ($1,$2,$3,$4,$5) \
             ON CONFLICT (review_id) DO NOTHING \
             RETURNING *",
        )
        .bind(d.review_id)
        .bind(&d.guild_id)
        .bind(&d.channel_id)
        .bind(&d.opened_by_id)
        .bind(&d.opened_by_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        if let Some(row) = inserted {
            return Ok((row.into(), true));
        }
        // Conflit : un salon existait deja -> on le renvoie.
        let existing: DiscussionRow =
            sqlx::query_as("SELECT * FROM automod_discussion_channels WHERE review_id = $1")
                .bind(d.review_id)
                .fetch_one(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok((existing.into(), false))
    }

    pub(super) async fn delete_discussion_impl(&self, review_id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM automod_discussion_channels WHERE review_id = $1")
            .bind(review_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    pub(super) async fn append_discussion_messages_impl(
        &self,
        messages: &[DiscussionMessage],
    ) -> Result<u64, DomainError> {
        if messages.is_empty() {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let mut inserted = 0u64;
        for m in messages {
            let res = sqlx::query(
                "INSERT INTO automod_discussion_messages \
                    (review_id, discord_message_id, author_id, author_name, author_is_bot, content, sent_at) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7) \
                 ON CONFLICT (review_id, discord_message_id) DO NOTHING",
            )
            .bind(m.review_id)
            .bind(&m.discord_message_id)
            .bind(&m.author_id)
            .bind(&m.author_name)
            .bind(m.author_is_bot)
            .bind(&m.content)
            .bind(m.sent_at)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;
            inserted += res.rows_affected();
        }
        tx.commit().await.map_err(pg_err)?;
        Ok(inserted)
    }

    pub(super) async fn list_discussion_messages_impl(
        &self,
        review_id: Uuid,
    ) -> Result<Vec<DiscussionMessage>, DomainError> {
        let rows: Vec<DiscussionMsgRow> = sqlx::query_as(
            "SELECT review_id, discord_message_id, author_id, author_name, author_is_bot, content, sent_at \
             FROM automod_discussion_messages WHERE review_id = $1 ORDER BY sent_at ASC",
        )
        .bind(review_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
