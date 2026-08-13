use async_trait::async_trait;

use crate::sentinel::domain::entities::system::analytics::*;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait AnalyticsRepository: Send + Sync {
    /// Heatmap : activite par heure et jour de la semaine.
    async fn get_heatmap(
        &self,
        guild_id: Option<&str>,
        days: i32,
    ) -> Result<Vec<HourlyActivity>, DomainError>;

    /// Distribution des actions (warn/delete/mute/ban).
    async fn get_action_distribution(
        &self,
        guild_id: Option<&str>,
        days: i32,
    ) -> Result<Vec<ActionDistribution>, DomainError>;

    /// Top infracteurs.
    /// `min_total` : seuil minimal d'infractions pour apparaître (0 = pas de filtre).
    async fn get_top_infractors(
        &self,
        guild_id: Option<&str>,
        days: i32,
        limit: i64,
        min_total: i64,
    ) -> Result<Vec<TopInfractor>, DomainError>;

    /// Trend moderation par jour.
    async fn get_moderation_trend(
        &self,
        guild_id: Option<&str>,
        days: i32,
    ) -> Result<Vec<ModerationTrend>, DomainError>;

    /// Pics d'activite (top heures).
    async fn get_peak_hours(
        &self,
        guild_id: Option<&str>,
        days: i32,
    ) -> Result<Vec<PeakActivity>, DomainError>;

    /// Enregistre l'activite d'une heure donnee (increment).
    async fn record_hourly(
        &self,
        guild_id: &str,
        hour: i16,
        messages: i64,
        infractions: i32,
    ) -> Result<(), DomainError>;

    /// Reset les compteurs d'activite (hourly_activity + daily_activity)
    /// pour une guild. NE TOUCHE PAS aux infractions/audit_logs (donnees
    /// d'audit reelles, conservees pour la chaine de moderation).
    /// Retourne le nombre total de lignes supprimees.
    async fn reset_activity(&self, guild_id: &str) -> Result<u64, DomainError>;
}
