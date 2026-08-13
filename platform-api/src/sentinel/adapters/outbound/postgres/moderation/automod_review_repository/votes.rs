use super::*;

impl PgAutomodReviewRepository {
    pub(super) async fn upsert_vote_impl(
        &self,
        review_id: Uuid,
        voter_id: &str,
        voter_name: &str,
        vote_action: &str,
    ) -> Result<(), DomainError> {
        // Refuse le vote si la review n'est plus ouverte.
        let status: Option<(String,)> =
            sqlx::query_as("SELECT status FROM automod_reviews WHERE id = $1")
                .bind(review_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        match status {
            None => {
                return Err(DomainError::NotFound(format!(
                    "review {review_id} introuvable"
                )))
            }
            Some((s,)) if s != "voting" => {
                return Err(DomainError::Conflict(format!("vote ferme (status={s})")))
            }
            _ => {}
        }

        sqlx::query(
            "INSERT INTO automod_review_votes (review_id, voter_id, voter_name, vote_action) \
             VALUES ($1,$2,$3,$4) \
             ON CONFLICT (review_id, voter_id) \
             DO UPDATE SET vote_action = EXCLUDED.vote_action, \
                           voter_name = EXCLUDED.voter_name, updated_at = NOW()",
        )
        .bind(review_id)
        .bind(voter_id)
        .bind(voter_name)
        .bind(vote_action)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    pub(super) async fn list_votes_impl(
        &self,
        review_id: Uuid,
    ) -> Result<Vec<ReviewVote>, DomainError> {
        let rows: Vec<VoteRow> = sqlx::query_as(
            "SELECT * FROM automod_review_votes WHERE review_id = $1 ORDER BY created_at",
        )
        .bind(review_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
