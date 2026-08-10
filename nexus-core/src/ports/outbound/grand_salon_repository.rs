//! Persistance du jeu Le Grand Salon.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::grand_salon::{
    Cercle, Dossier, GazetteArticle, Habitué, MotionDuSalon,
};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait GrandSalonRepository: Send + Sync {
    async fn find_habitue(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Habitué>, DomainError>;
    async fn save_habitue(&self, habitue: &Habitué) -> Result<(), DomainError>;
    async fn claim_daily(&self, habitue_id: Uuid) -> Result<bool, DomainError>;
    async fn create_cercle(&self, cercle: &Cercle) -> Result<(), DomainError>;
    async fn list_cercles(&self, guild_id: &str) -> Result<Vec<Cercle>, DomainError>;
    async fn create_motion(&self, motion: &MotionDuSalon) -> Result<(), DomainError>;
    async fn list_motions(&self, guild_id: &str) -> Result<Vec<MotionDuSalon>, DomainError>;
    async fn cast_vote(
        &self,
        motion_id: Uuid,
        habitue_id: Uuid,
        choice: bool,
        weight: i64,
    ) -> Result<(), DomainError>;
    async fn vote_totals(&self, motion_id: Uuid) -> Result<(i64, i64), DomainError>;
    async fn due_motions(&self) -> Result<Vec<MotionDuSalon>, DomainError>;
    async fn close_motion(&self, id: Uuid, adopted: bool) -> Result<(), DomainError>;
    async fn publish_gazette(&self, article: &GazetteArticle) -> Result<(), DomainError>;
    async fn list_gazette(&self, guild_id: &str) -> Result<Vec<GazetteArticle>, DomainError>;
    async fn create_dossier(&self, dossier: &Dossier) -> Result<(), DomainError>;
    async fn list_dossiers(
        &self,
        guild_id: &str,
        owner_id: Uuid,
    ) -> Result<Vec<Dossier>, DomainError>;
    async fn reveal_dossier(&self, dossier_id: Uuid, owner_id: Uuid) -> Result<(), DomainError>;
}
