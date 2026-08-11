use super::*;

impl PgAutomodReviewRepository {
    pub(super) async fn create_impl(
        &self,
        r: NewAutomodReview,
    ) -> Result<AutomodReview, DomainError> {
        // Mode vote si une echeance est fournie : statut 'voting'.
        let status = if r.voting_deadline.is_some() {
            "voting"
        } else {
            "pending"
        };
        let incidents = serde_json::json!([incident_json(&r)]);
        let row: Row = sqlx::query_as(
            "INSERT INTO automod_reviews \
                (guild_id, channel_id, message_id, user_id, user_name, content_preview, \
                 suggested_action, score, reason, flags, status, voting_deadline, \
                 incident_count, cumulative_score, incidents, sanction_logged) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,1,$13,$14,$15) \
             RETURNING *",
        )
        .bind(r.guild_id.as_str())
        .bind(r.channel_id.as_str())
        .bind(r.message_id.as_str())
        .bind(r.user_id.as_str())
        .bind(&r.user_name)
        .bind(&r.content_preview)
        .bind(r.suggested_action.as_str())
        .bind(r.score)
        .bind(&r.reason)
        .bind(&r.flags)
        .bind(status)
        .bind(r.voting_deadline)
        .bind(r.score)
        .bind(&incidents)
        .bind(r.sanction_logged)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.into())
    }

    pub(super) async fn create_or_merge_impl(
        &self,
        r: NewAutomodReview,
        aggregate: bool,
        window_minutes: i64,
    ) -> Result<(AutomodReview, bool), DomainError> {
        if aggregate {
            // Fenetre d'inactivite : on ne fusionne que dans une carte ayant eu
            // une infraction recemment. 0/negatif => pas de limite (legacy).
            let window = window_minutes.max(0);
            // Serialise les agregations concurrentes du meme (guild, user) :
            // sans ca, deux messages quasi simultanes pourraient creer 2 cartes
            // ou perdre un incident (read-modify-write sur le tableau JSON).
            let mut tx = self.pool.begin().await.map_err(pg_err)?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
                .bind(format!(
                    "automod_review:{}:{}",
                    r.guild_id.as_str(),
                    r.user_id.as_str()
                ))
                .execute(&mut *tx)
                .await
                .map_err(pg_err)?;

            // Carte ouverte existante pour ce (guild, user) ET active (dernier
            // incident dans la fenetre). Si window = 0 -> pas de filtre temporel.
            let existing: Option<Row> = sqlx::query_as(
                "SELECT * FROM automod_reviews \
                 WHERE guild_id = $1 AND user_id = $2 AND status = 'voting' \
                   AND ($3 = 0 OR last_incident_at > NOW() - make_interval(mins => $3)) \
                 ORDER BY last_incident_at DESC LIMIT 1",
            )
            .bind(r.guild_id.as_str())
            .bind(r.user_id.as_str())
            .bind(window as i32)
            .fetch_optional(&mut *tx)
            .await
            .map_err(pg_err)?;

            if let Some(prev) = existing {
                // Agrege l'incident dans la carte existante.
                let mut incidents = if prev.incidents.is_null() {
                    serde_json::json!([])
                } else {
                    prev.incidents.clone()
                };
                if let Some(arr) = incidents.as_array_mut() {
                    arr.push(incident_json(&r));
                }
                let new_count = prev.incident_count + 1;
                let new_cumulative = prev.cumulative_score + r.score;
                let new_max_score = prev.score.max(r.score);
                let incident_action = sentinel_core::domain::entities::moderation::review::automod::more_severe_suggested(
                    &prev.suggested_action,
                    r.suggested_action.as_str(),
                );
                let new_action = sentinel_core::domain::entities::moderation::review::automod::more_severe_suggested(
                    &incident_action,
                    action_for_cumulative_score(new_cumulative),
                );
                // Plafond anti-troll : la deadline ne peut etre repoussee au-dela
                // de created_at + 7 jours (un membre tres actif ne garde pas la
                // carte ouverte indefiniment).
                let cap = prev.created_at + chrono::Duration::days(7);
                let new_deadline = r.voting_deadline.map(|d| d.min(cap));
                let updated: Row = sqlx::query_as(
                    "UPDATE automod_reviews SET \
                        incidents = $1, incident_count = $2, cumulative_score = $3, \
                        score = $4, suggested_action = $5, reason = $6, voting_deadline = $7, \
                        content_preview = $9, channel_id = $10, message_id = $11, \
                        sanction_logged = sanction_logged OR $12, \
                        last_incident_at = NOW() \
                     WHERE id = $8 AND status = 'voting' \
                     RETURNING *",
                )
                .bind(&incidents)
                .bind(new_count)
                .bind(new_cumulative)
                .bind(new_max_score)
                .bind(&new_action)
                .bind(&r.reason)
                .bind(new_deadline)
                .bind(prev.id)
                .bind(&r.content_preview)
                .bind(r.channel_id.as_str())
                .bind(r.message_id.as_str())
                .bind(r.sanction_logged)
                .fetch_one(&mut *tx)
                .await
                .map_err(pg_err)?;
                tx.commit().await.map_err(pg_err)?;
                return Ok((updated.into(), true));
            }

            // Aucune carte ouverte : on cree dans la meme transaction (sous le
            // verrou) pour eviter une creation concurrente en double.
            let status = if r.voting_deadline.is_some() {
                "voting"
            } else {
                "pending"
            };
            let incidents = serde_json::json!([incident_json(&r)]);
            let row: Row = sqlx::query_as(
                "INSERT INTO automod_reviews \
                    (guild_id, channel_id, message_id, user_id, user_name, content_preview, \
                     suggested_action, score, reason, flags, status, voting_deadline, \
                     incident_count, cumulative_score, incidents) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,1,$13,$14) \
                 RETURNING *",
            )
            .bind(r.guild_id.as_str())
            .bind(r.channel_id.as_str())
            .bind(r.message_id.as_str())
            .bind(r.user_id.as_str())
            .bind(&r.user_name)
            .bind(&r.content_preview)
            .bind(r.suggested_action.as_str())
            .bind(r.score)
            .bind(&r.reason)
            .bind(&r.flags)
            .bind(status)
            .bind(r.voting_deadline)
            .bind(r.score)
            .bind(&incidents)
            .bind(r.sanction_logged)
            .fetch_one(&mut *tx)
            .await
            .map_err(pg_err)?;
            tx.commit().await.map_err(pg_err)?;
            return Ok((row.into(), false));
        }
        // Pas d'agregation : creation normale.
        let review = self.create(r).await?;
        Ok((review, false))
    }

    pub(super) async fn get_impl(&self, id: Uuid) -> Result<Option<AutomodReview>, DomainError> {
        let row: Option<Row> = sqlx::query_as("SELECT * FROM automod_reviews WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    pub(super) async fn find_by_message_id_impl(
        &self,
        guild_id: &str,
        message_id: &str,
    ) -> Result<Option<AutomodReview>, DomainError> {
        let row: Option<Row> = sqlx::query_as(
            "SELECT * FROM automod_reviews \
             WHERE guild_id = $1 AND message_id = $2 \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(guild_id)
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    pub(super) async fn list_pending_impl(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT * FROM automod_reviews \
             WHERE guild_id = $1 AND status = 'pending' \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub(super) async fn list_recent_impl(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT * FROM automod_reviews \
             WHERE guild_id = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
