//! Port sortant du copilote de moderation : agregations en LECTURE SEULE.
//!
//! Regroupe les requetes propres au copilote (historique de sanctions par type,
//! derniere sanction, reviews ouvertes, jurisprudence par categorie de flag)
//! dans un port focalise, afin de ne pas alourdir les repos existants ni casser
//! leurs mocks. L'agregation de jurisprudence EXCLUT les reviews en statut
//! `voting` (anti-ancrage : seules les decisions deja tranchees comptent).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;

use crate::sentinel::domain::entities::moderation::copilot::PrecedentDistribution;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ModerationCopilotRepository: Send + Sync {
    /// Nombre de sanctions appliquees par type pour un membre depuis `since`
    /// (`action_type -> nombre`). Trie par nombre decroissant.
    async fn count_sanctions_by_type(
        &self,
        guild_id: &str,
        user_id: &str,
        since: DateTime<Utc>,
    ) -> Result<Vec<(String, u32)>, DomainError>;

    /// Date de la derniere sanction appliquee au membre (toutes fenetres), le
    /// cas echeant.
    async fn last_sanction_at(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError>;

    /// Nombre de reviews automod encore OUVERTES (statut `voting`|`pending`|
    /// `decided`) visant ce membre.
    async fn count_open_reviews(&self, guild_id: &str, user_id: &str) -> Result<u32, DomainError>;

    /// Categorie de flag DOMINANTE du membre sur ses reviews recentes tranchees
    /// (depuis `since`, hors `voting`). `None` si aucune review exploitable.
    async fn dominant_flag_category(
        &self,
        guild_id: &str,
        user_id: &str,
        since: DateTime<Utc>,
    ) -> Result<Option<String>, DomainError>;

    /// Jurisprudence : distribution des actions retenues sur les reviews de la
    /// guild portant `flag_category`, depuis `since`. CRITIQUE : exclut les
    /// lignes `status = 'voting'` (anti-ancrage). Prefere `applied_action`
    /// quand present, sinon `decided_action`.
    async fn aggregate_decided_by_flag(
        &self,
        guild_id: &str,
        flag_category: &str,
        since: DateTime<Utc>,
    ) -> Result<PrecedentDistribution, DomainError>;
}
