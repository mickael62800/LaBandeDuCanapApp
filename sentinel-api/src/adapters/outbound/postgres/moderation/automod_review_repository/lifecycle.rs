use super::*;

impl PgAutomodReviewRepository {
    pub(super) async fn resolve_impl(
        &self,
        id: Uuid,
        applied_action: &str,
        resolved_by_id: &str,
        resolved_by_name: &str,
        resolved_source: &str,
    ) -> Result<AutomodReview, DomainError> {
        let new_status = if applied_action == "ignore" {
            "ignored"
        } else {
            "applied"
        };
        let row: Option<Row> = sqlx::query_as(
            "UPDATE automod_reviews SET \
                status = $1, applied_action = $2, resolved_by_id = $3, \
                resolved_by_name = $4, resolved_source = $5, resolved_at = NOW() \
             WHERE id = $6 AND status IN ('pending','decided') \
             RETURNING *",
        )
        .bind(new_status)
        .bind(applied_action)
        .bind(resolved_by_id)
        .bind(resolved_by_name)
        .bind(resolved_source)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        match row {
            Some(r) => Ok(r.into()),
            None => {
                // Soit la review n existe pas, soit deja resolue.
                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT status FROM automod_reviews WHERE id = $1")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(pg_err)?;
                match exists {
                    None => Err(DomainError::NotFound(format!("review {id} introuvable"))),
                    Some((s,)) => Err(DomainError::Conflict(format!(
                        "review deja resolue (status={s})"
                    ))),
                }
            }
        }
    }

    pub(super) async fn close_ignored_impl(
        &self,
        id: Uuid,
        actor_id: &str,
        actor_name: &str,
        source: &str,
    ) -> Result<AutomodReview, DomainError> {
        // Clore immediatement, meme pendant le vote (statut voting inclus).
        let row: Option<Row> = sqlx::query_as(
            "UPDATE automod_reviews SET \
                status = 'ignored', applied_action = 'ignore', resolved_by_id = $2, \
                resolved_by_name = $3, resolved_source = $4, resolved_at = NOW() \
             WHERE id = $1 AND status IN ('pending','voting','decided') \
             RETURNING *",
        )
        .bind(id)
        .bind(actor_id)
        .bind(actor_name)
        .bind(source)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        match row {
            Some(r) => Ok(r.into()),
            None => {
                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT status FROM automod_reviews WHERE id = $1")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(pg_err)?;
                match exists {
                    None => Err(DomainError::NotFound(format!("review {id} introuvable"))),
                    Some((s,)) => Err(DomainError::Conflict(format!(
                        "review deja close (status={s})"
                    ))),
                }
            }
        }
    }

    pub(super) async fn reopen_impl(
        &self,
        id: Uuid,
        deadline_hours: i64,
    ) -> Result<AutomodReview, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Repasse en 'voting' : efface la resolution + le verdict + fixe une
        // nouvelle echeance. Seules les reviews closes (applied|ignored) sont
        // rouvrables.
        let row: Option<Row> = sqlx::query_as(
            "UPDATE automod_reviews SET \
                status = 'voting', applied_action = NULL, decided_action = NULL, \
                quorum_met = FALSE, decided_at = NULL, resolved_by_id = NULL, \
                resolved_by_name = NULL, resolved_source = NULL, resolved_at = NULL, \
                voting_deadline = NOW() + make_interval(hours => $2), \
                sanction_logged = (status = 'applied') \
             WHERE id = $1 AND status IN ('applied','ignored') \
             RETURNING *",
        )
        .bind(id)
        .bind(deadline_hours as i32)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;

        let review = match row {
            Some(r) => r,
            None => {
                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT status FROM automod_reviews WHERE id = $1")
                        .bind(id)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(pg_err)?;
                tx.rollback().await.map_err(pg_err)?;
                return match exists {
                    None => Err(DomainError::NotFound(format!("review {id} introuvable"))),
                    Some((s,)) => Err(DomainError::Conflict(format!(
                        "dossier non rouvrable (status={s})"
                    ))),
                };
            }
        };

        // Repart sur un vote propre : on efface les votes du tour precedent.
        sqlx::query("DELETE FROM automod_review_votes WHERE review_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(review.into())
    }

    pub(super) async fn decide_impl(
        &self,
        id: Uuid,
        decided_action: &str,
        quorum_met: bool,
    ) -> Result<AutomodReview, DomainError> {
        let row: Option<Row> = sqlx::query_as(
            "UPDATE automod_reviews SET \
                status = 'decided', decided_action = $1, quorum_met = $2, decided_at = NOW() \
             WHERE id = $3 AND status = 'voting' \
             RETURNING *",
        )
        .bind(decided_action)
        .bind(quorum_met)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        match row {
            Some(r) => Ok(r.into()),
            None => {
                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT status FROM automod_reviews WHERE id = $1")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(pg_err)?;
                match exists {
                    None => Err(DomainError::NotFound(format!("review {id} introuvable"))),
                    Some((s,)) => Err(DomainError::Conflict(format!(
                        "vote deja cloture (status={s})"
                    ))),
                }
            }
        }
    }

    pub(super) async fn list_expired_voting_impl(
        &self,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT * FROM automod_reviews \
             WHERE status = 'voting' AND voting_deadline IS NOT NULL AND voting_deadline < NOW() \
             ORDER BY voting_deadline ASC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub(super) async fn fp_terminal_reviews_impl(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<FpTerminalReview>, DomainError> {
        #[derive(sqlx::FromRow)]
        struct TerminalRow {
            suggested_action: String,
            applied_action: Option<String>,
            decided_action: Option<String>,
            flags: serde_json::Value,
        }
        let rows: Vec<TerminalRow> = sqlx::query_as(
            "SELECT suggested_action, applied_action, decided_action, flags \
             FROM automod_reviews \
             WHERE guild_id = $1 \
               AND status IN ('applied','ignored','decided') \
               AND created_at >= NOW() - make_interval(days => $2) \
             ORDER BY created_at DESC \
             LIMIT $3",
        )
        .bind(guild_id)
        .bind(days as i32)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|r| FpTerminalReview {
                suggested_action: r.suggested_action,
                applied_action: r.applied_action,
                decided_action: r.decided_action,
                flags: r.flags,
            })
            .collect())
    }

    pub(super) async fn expire_stale_decided_impl(
        &self,
        grace_hours: i64,
        limit: i64,
    ) -> Result<Vec<ExpiredReviewCard>, DomainError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            action_id: Uuid,
            channel_id: String,
            message_id: String,
        }
        // Passe les 'decided' trop vieux en 'ignored' (le verdict lapse faute de
        // finalisation admin) et renvoie leurs cartes a nettoyer. Le mapping de
        // carte est retire dans la meme CTE.
        let rows: Vec<Row> = sqlx::query_as(
            "WITH to_expire AS ( \
                 SELECT id FROM automod_reviews \
                 WHERE status = 'decided' \
                   AND decided_at IS NOT NULL \
                   AND decided_at < NOW() - make_interval(hours => $1) \
                 LIMIT $2 \
             ), expired AS ( \
                 UPDATE automod_reviews SET status = 'ignored', resolved_at = NOW(), \
                     resolved_source = 'auto_expired' \
                 WHERE id IN (SELECT id FROM to_expire) \
                 RETURNING id \
             ), cards AS ( \
                 DELETE FROM discord_action_messages m \
                 USING expired e \
                 WHERE m.action_id = e.id AND m.kind = 'automod_review' \
                 RETURNING m.action_id, m.channel_id, m.message_id \
             ) \
             SELECT action_id, channel_id, message_id FROM cards",
        )
        .bind(grace_hours as i32)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| ExpiredReviewCard {
                action_id: r.action_id,
                channel_id: r.channel_id,
                message_id: r.message_id,
            })
            .collect())
    }

    pub(super) async fn expire_review_cards_impl(
        &self,
        days: i64,
        limit: i64,
    ) -> Result<Vec<ExpiredReviewCard>, DomainError> {
        #[derive(sqlx::FromRow)]
        struct ExpiredRow {
            action_id: Uuid,
            channel_id: String,
            message_id: String,
        }
        let rows: Vec<ExpiredRow> = sqlx::query_as(
            "SELECT m.action_id, m.channel_id, m.message_id \
             FROM automod_reviews r \
             JOIN discord_action_messages m ON m.action_id = r.id AND m.kind = 'automod_review' \
             WHERE r.status IN ('applied','ignored') \
               AND r.resolved_at IS NOT NULL \
               AND r.resolved_at < NOW() - make_interval(days => $1) \
             LIMIT $2",
        )
        .bind(days as i32)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        // Retire le mapping pour ne pas re-traiter au prochain passage.
        for row in &rows {
            let _ = sqlx::query(
                "DELETE FROM discord_action_messages WHERE action_id = $1 AND kind = 'automod_review'",
            )
            .bind(row.action_id)
            .execute(&self.pool)
            .await;
        }

        Ok(rows
            .into_iter()
            .map(|r| ExpiredReviewCard {
                action_id: r.action_id,
                channel_id: r.channel_id,
                message_id: r.message_id,
            })
            .collect())
    }
}
