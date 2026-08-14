//! Adaptateur PostgreSQL des hauts faits (migration 031_achievements.sql).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_core::nexus::{
    domain::{
        entities::achievement::{
            Achievement, GameIdentity, GamePlayerLink, UserAchievement, Verification,
        },
        errors::DomainError,
    },
    ports::outbound::achievement_repository::{AchievementRepository, AchievementUpdate},
};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

use super::pg_ctx;

const DEF_COLS: &str =
    "id, game, code, name, description, category, icon_url, criteria, verification, hidden, enabled";

pub struct PgAchievementRepository {
    pool: PgPool,
}

impl PgAchievementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn definition_from_row(row: &PgRow) -> Result<Achievement, DomainError> {
    Ok(Achievement {
        id: row.get("id"),
        game: row.get("game"),
        code: row.get("code"),
        name: row.get("name"),
        description: row.get("description"),
        category: row.get("category"),
        icon_url: row.get("icon_url"),
        criteria: row.get("criteria"),
        verification: Verification::parse(row.get::<&str, _>("verification"))?,
        hidden: row.get("hidden"),
        enabled: row.get("enabled"),
    })
}

fn link_from_row(row: &PgRow) -> GamePlayerLink {
    GamePlayerLink {
        id: row.get("id"),
        guild_id: row.get("guild_id"),
        discord_user_id: row.get("discord_user_id"),
        game: row.get("game"),
        game_player_id: row.get("game_player_id"),
        verified_at: row.get("verified_at"),
    }
}

#[async_trait]
impl AchievementRepository for PgAchievementRepository {
    async fn list_definitions(&self, game: Option<&str>) -> Result<Vec<Achievement>, DomainError> {
        // `game IS NOT DISTINCT FROM` gere le cas transverse (game NULL) sans
        // requete separee : passer None filtre alors sur les hauts faits
        // globaux, et l'absence de filtre renvoie tout.
        let rows = match game {
            Some(game) => {
                sqlx::query(&format!(
                    "SELECT {DEF_COLS} FROM achievements WHERE game = $1 ORDER BY category, name"
                ))
                .bind(game)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query(&format!(
                    "SELECT {DEF_COLS} FROM achievements ORDER BY game NULLS FIRST, category, name"
                ))
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(pg_ctx("list_definitions"))?;

        rows.iter().map(definition_from_row).collect()
    }

    async fn find_definition(&self, id: Uuid) -> Result<Option<Achievement>, DomainError> {
        let row = sqlx::query(&format!(
            "SELECT {DEF_COLS} FROM achievements WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("find_definition"))?;
        row.as_ref().map(definition_from_row).transpose()
    }

    async fn find_definition_by_code(
        &self,
        game: Option<&str>,
        code: &str,
    ) -> Result<Option<Achievement>, DomainError> {
        let row = sqlx::query(&format!(
            "SELECT {DEF_COLS} FROM achievements \
             WHERE game IS NOT DISTINCT FROM $1 AND code = $2"
        ))
        .bind(game)
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("find_definition_by_code"))?;
        row.as_ref().map(definition_from_row).transpose()
    }

    async fn update_definition(
        &self,
        id: Uuid,
        update: AchievementUpdate,
    ) -> Result<Achievement, DomainError> {
        // COALESCE par champ : un `None` laisse la valeur en base intacte.
        // `icon_url` est un Option<Option<_>> — l'appelant peut donc EFFACER
        // l'image (Some(None)) sans que ce soit confondu avec « ne pas
        // toucher » (None). D'ou le drapeau `$2` distinct de la valeur `$3`.
        let (set_icon, icon_value) = match update.icon_url {
            Some(value) => (true, value),
            None => (false, None),
        };

        let row = sqlx::query(&format!(
            "UPDATE achievements SET \
               icon_url = CASE WHEN $2 THEN $3 ELSE icon_url END, \
               name = COALESCE($4, name), \
               description = COALESCE($5, description), \
               enabled = COALESCE($6, enabled), \
               hidden = COALESCE($7, hidden), \
               criteria = COALESCE($8, criteria) \
             WHERE id = $1 RETURNING {DEF_COLS}"
        ))
        .bind(id)
        .bind(set_icon)
        .bind(
            icon_value
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
        )
        .bind(update.name)
        .bind(update.description)
        .bind(update.enabled)
        .bind(update.hidden)
        .bind(update.criteria)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("update_definition"))?
        .ok_or_else(|| DomainError::NotFound("haut fait introuvable".into()))?;

        definition_from_row(&row)
    }

    async fn find_link(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: &str,
    ) -> Result<Option<GamePlayerLink>, DomainError> {
        let row = sqlx::query(
            "SELECT id, guild_id, discord_user_id, game, game_player_id, verified_at \
             FROM game_player_links \
             WHERE guild_id = $1 AND discord_user_id = $2 AND game = $3",
        )
        .bind(guild_id)
        .bind(discord_user_id)
        .bind(game)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("find_link"))?;
        Ok(row.as_ref().map(link_from_row))
    }

    async fn find_link_by_player(
        &self,
        guild_id: &str,
        identity: &GameIdentity,
    ) -> Result<Option<GamePlayerLink>, DomainError> {
        let row = sqlx::query(
            "SELECT id, guild_id, discord_user_id, game, game_player_id, verified_at \
             FROM game_player_links \
             WHERE guild_id = $1 AND game = $2 AND game_player_id = $3",
        )
        .bind(guild_id)
        .bind(identity.game())
        .bind(identity.player_id())
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("find_link_by_player"))?;
        Ok(row.as_ref().map(link_from_row))
    }

    async fn upsert_link(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        identity: &GameIdentity,
        verified: bool,
    ) -> Result<GamePlayerLink, DomainError> {
        let verified_at = verified.then(Utc::now);
        let row = sqlx::query(
            "INSERT INTO game_player_links \
                (id, guild_id, discord_user_id, game, game_player_id, verified_at) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (guild_id, game, discord_user_id) DO UPDATE SET \
                game_player_id = EXCLUDED.game_player_id, \
                verified_at = EXCLUDED.verified_at \
             RETURNING id, guild_id, discord_user_id, game, game_player_id, verified_at",
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(discord_user_id)
        .bind(identity.game())
        .bind(identity.player_id())
        .bind(verified_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            // L'autre unicite — (guild, game, game_player_id) — protege contre
            // l'usurpation. Elle n'est PAS couverte par le ON CONFLICT
            // ci-dessus : on traduit sa violation en conflit metier lisible
            // plutot qu'en erreur d'infrastructure opaque.
            if matches!(&e, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505")) {
                DomainError::Conflict(
                    "cette identite de jeu est deja liee a un autre membre".into(),
                )
            } else {
                DomainError::Infrastructure(format!("upsert_link pg: {e}"))
            }
        })?;
        Ok(link_from_row(&row))
    }

    async fn delete_link(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: &str,
    ) -> Result<bool, DomainError> {
        let result = sqlx::query(
            "DELETE FROM game_player_links \
             WHERE guild_id = $1 AND discord_user_id = $2 AND game = $3",
        )
        .bind(guild_id)
        .bind(discord_user_id)
        .bind(game)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("delete_link"))?;
        Ok(result.rows_affected() > 0)
    }

    async fn insert_unlock(
        &self,
        unlock: &UserAchievement,
    ) -> Result<Option<UserAchievement>, DomainError> {
        // DO NOTHING sur les DEUX unicites (deja possede / evenement rejoue) :
        // `fetch_optional` renvoie alors None, ce que le service traduit en
        // « rien a publier ». L'idempotence est portee par la base, pas par un
        // SELECT prealable qui laisserait une fenetre de course.
        let row = sqlx::query(
            "INSERT INTO user_achievements \
                (id, guild_id, discord_user_id, achievement_id, game_player_id, \
                 source_event_id, granted_by, unlocked_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             ON CONFLICT DO NOTHING \
             RETURNING id, guild_id, discord_user_id, achievement_id, game_player_id, \
                       source_event_id, granted_by, unlocked_at",
        )
        .bind(unlock.id)
        .bind(&unlock.guild_id)
        .bind(&unlock.discord_user_id)
        .bind(unlock.achievement_id)
        .bind(unlock.game_player_id.as_deref())
        .bind(unlock.source_event_id.as_deref())
        .bind(unlock.granted_by.as_deref())
        .bind(unlock.unlocked_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("insert_unlock"))?;

        Ok(row.map(|row| UserAchievement {
            id: row.get("id"),
            guild_id: row.get("guild_id"),
            discord_user_id: row.get("discord_user_id"),
            achievement_id: row.get("achievement_id"),
            game_player_id: row.get("game_player_id"),
            source_event_id: row.get("source_event_id"),
            granted_by: row.get("granted_by"),
            unlocked_at: row.get::<DateTime<Utc>, _>("unlocked_at"),
        }))
    }

    async fn list_for_member(
        &self,
        guild_id: &str,
        discord_user_id: &str,
    ) -> Result<Vec<UserAchievement>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, guild_id, discord_user_id, achievement_id, game_player_id, \
                    source_event_id, granted_by, unlocked_at \
             FROM user_achievements \
             WHERE guild_id = $1 AND discord_user_id = $2 \
             ORDER BY unlocked_at DESC",
        )
        .bind(guild_id)
        .bind(discord_user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("list_for_member"))?;

        Ok(rows
            .into_iter()
            .map(|row| UserAchievement {
                id: row.get("id"),
                guild_id: row.get("guild_id"),
                discord_user_id: row.get("discord_user_id"),
                achievement_id: row.get("achievement_id"),
                game_player_id: row.get("game_player_id"),
                source_event_id: row.get("source_event_id"),
                granted_by: row.get("granted_by"),
                unlocked_at: row.get::<DateTime<Utc>, _>("unlocked_at"),
            })
            .collect())
    }

    async fn count_for_member(
        &self,
        guild_id: &str,
        discord_user_id: &str,
    ) -> Result<i64, DomainError> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_achievements WHERE guild_id = $1 AND discord_user_id = $2",
        )
        .bind(guild_id)
        .bind(discord_user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx("count_for_member"))?;
        Ok(count)
    }
}
