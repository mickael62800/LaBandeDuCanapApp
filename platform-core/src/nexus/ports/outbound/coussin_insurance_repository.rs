use crate::nexus::domain::errors::DomainError;
use async_trait::async_trait;
#[derive(Debug, Clone)]
pub struct CoussinInsurance {
    pub is_scam: bool,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}
#[async_trait]
pub trait CoussinInsuranceRepository: Send + Sync {
    /// Le prix et la duree sont PASSES : le depot ne va pas les chercher.
    /// Ils etaient ecrits dans la requete SQL, ce qui rendait le reglage
    /// « Prix de la garantie » purement decoratif.
    async fn buy(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        cost: i64,
        duration_minutes: i64,
    ) -> Result<CoussinInsurance, DomainError>;
    async fn active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoussinInsurance>, DomainError>;
}
