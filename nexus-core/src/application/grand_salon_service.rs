//! Cas d'usage du Grand Salon, sans dépendance Discord ou SQL.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::grand_salon::{
    Cercle, CercleKind, Dossier, GazetteArticle, Habitué, MotionDuSalon, MotionStatus, Ressources,
};
use crate::domain::errors::DomainError;
use crate::ports::outbound::grand_salon_repository::GrandSalonRepository;

pub struct GrandSalonService {
    repo: Arc<dyn GrandSalonRepository>,
    starting_jetons: i64,
}

impl GrandSalonService {
    pub fn new(repo: Arc<dyn GrandSalonRepository>, starting_jetons: i64) -> Self {
        Self {
            repo,
            starting_jetons,
        }
    }

    /// Inscription idempotente : un habitué existant n'est jamais réinitialisé.
    pub async fn join(
        &self,
        guild_id: &str,
        user_id: &str,
        display_name: &str,
        now: DateTime<Utc>,
    ) -> Result<Habitué, DomainError> {
        if guild_id.trim().is_empty() || user_id.trim().is_empty() || display_name.trim().is_empty()
        {
            return Err(DomainError::ValidationError(
                "guild_id, user_id et display_name sont obligatoires".into(),
            ));
        }
        if let Some(existing) = self.repo.find_habitue(guild_id, user_id).await? {
            return Ok(existing);
        }
        let habitue = Habitué {
            id: Uuid::new_v4(),
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            display_name: display_name.trim().into(),
            ressources: Ressources::newcomer(self.starting_jetons),
            joined_at: now,
        };
        self.repo.save_habitue(&habitue).await?;
        Ok(habitue)
    }

    pub async fn propose_motion(&self, motion: MotionDuSalon) -> Result<(), DomainError> {
        if motion.titre.trim().is_empty() || motion.texte.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "une motion doit avoir un titre et un texte".into(),
            ));
        }
        if motion.status != MotionStatus::EnVote {
            return Err(DomainError::ValidationError(
                "seule une motion en vote peut etre proposee".into(),
            ));
        }
        self.repo.create_motion(&motion).await
    }

    pub async fn profile(&self, guild_id: &str, user_id: &str) -> Result<Habitué, DomainError> {
        self.repo
            .find_habitue(guild_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("habitué du Grand Salon introuvable".into()))
    }

    pub async fn daily(&self, guild_id: &str, user_id: &str) -> Result<Habitué, DomainError> {
        let habitue = self.profile(guild_id, user_id).await?;
        if !self.repo.claim_daily(habitue.id).await? {
            return Err(DomainError::ValidationError(
                "tu as deja participe au Salon aujourd'hui".into(),
            ));
        }
        self.profile(guild_id, user_id).await
    }

    pub async fn motions(&self, guild_id: &str) -> Result<Vec<MotionDuSalon>, DomainError> {
        self.repo.list_motions(guild_id).await
    }

    pub async fn create_cercle(
        &self,
        guild_id: &str,
        user_id: &str,
        kind: CercleKind,
        name: &str,
        devise: &str,
        now: DateTime<Utc>,
    ) -> Result<Cercle, DomainError> {
        if name.trim().chars().count() < 3 || name.trim().chars().count() > 60 {
            return Err(DomainError::ValidationError(
                "le nom du cercle doit contenir 3 a 60 caracteres".into(),
            ));
        }
        let founder = self.profile(guild_id, user_id).await?;
        let cercle = Cercle {
            id: Uuid::new_v4(),
            guild_id: guild_id.into(),
            kind,
            name: name.trim().into(),
            devise: devise.trim().chars().take(160).collect(),
            caisse: 0,
            reputation: 0,
            rayonnement: 0,
            founder_id: founder.id,
            created_at: now,
            dissolved_at: None,
        };
        self.repo.create_cercle(&cercle).await?;
        Ok(cercle)
    }

    pub async fn cercles(&self, guild_id: &str) -> Result<Vec<Cercle>, DomainError> {
        self.repo.list_cercles(guild_id).await
    }

    pub async fn investigate(
        &self,
        guild_id: &str,
        user_id: &str,
        subject: &str,
    ) -> Result<Dossier, DomainError> {
        if subject.trim().chars().count() < 3 {
            return Err(DomainError::ValidationError(
                "le sujet du dossier est trop court".into(),
            ));
        }
        let owner = self.profile(guild_id, user_id).await?;
        let dossier = Dossier {
            id: Uuid::new_v4(),
            guild_id: guild_id.into(),
            owner_id: owner.id,
            subject: subject.trim().chars().take(200).collect(),
            verified: owner.ressources.bons_plans + owner.ressources.reseau >= 100,
            revealed_at: None,
        };
        self.repo.create_dossier(&dossier).await?;
        Ok(dossier)
    }

    pub async fn dossiers(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<Dossier>, DomainError> {
        let owner = self.profile(guild_id, user_id).await?;
        self.repo.list_dossiers(guild_id, owner.id).await
    }

    pub async fn reveal(
        &self,
        guild_id: &str,
        user_id: &str,
        dossier_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let owner = self.profile(guild_id, user_id).await?;
        let dossier = self
            .repo
            .list_dossiers(guild_id, owner.id)
            .await?
            .into_iter()
            .find(|d| d.id == dossier_id)
            .ok_or_else(|| DomainError::NotFound("dossier introuvable".into()))?;
        if !dossier.verified {
            return Err(DomainError::ValidationError(
                "ce dossier n'est pas assez fiable pour la Gazette".into(),
            ));
        }
        self.repo.reveal_dossier(dossier_id, owner.id).await?;
        self.repo
            .publish_gazette(&GazetteArticle {
                id: Uuid::new_v4(),
                guild_id: guild_id.into(),
                headline: format!("La Gazette revele : {}", dossier.subject),
                body: "Un dossier vérifié vient d'être rendu public dans le Grand Salon.".into(),
                published_at: now,
            })
            .await
    }

    pub async fn vote(
        &self,
        guild_id: &str,
        user_id: &str,
        motion_id: Uuid,
        choice: bool,
    ) -> Result<(), DomainError> {
        let habitue = self.profile(guild_id, user_id).await?;
        let motion = self
            .repo
            .list_motions(guild_id)
            .await?
            .into_iter()
            .find(|motion| motion.id == motion_id)
            .ok_or_else(|| DomainError::NotFound("motion introuvable".into()))?;
        if motion.status != MotionStatus::EnVote || motion.closes_at <= Utc::now() {
            return Err(DomainError::ValidationError("le vote est clos".into()));
        }
        let weight = 1 + (habitue.ressources.rayonnement.max(0) / 500).min(4);
        self.repo
            .cast_vote(motion_id, habitue.id, choice, weight)
            .await
    }

    pub async fn gazette(&self, guild_id: &str) -> Result<Vec<GazetteArticle>, DomainError> {
        self.repo.list_gazette(guild_id).await
    }

    /// Clôture les motions échues et publie leur résultat dans la Gazette.
    pub async fn close_due_motions(
        &self,
        votes: &[(Uuid, i64, i64)],
        now: DateTime<Utc>,
    ) -> Result<usize, DomainError> {
        let mut closed = 0;
        for motion in self.repo.due_motions().await? {
            let supplied = votes
                .iter()
                .find(|(id, _, _)| *id == motion.id)
                .map(|(_, f, a)| (*f, *a))
                .unwrap_or(self.repo.vote_totals(motion.id).await?);
            let (for_votes, against_votes) = supplied;
            let adopted = motion.should_pass(for_votes, against_votes);
            self.repo.close_motion(motion.id, adopted).await?;
            let verdict = if adopted { "adoptée" } else { "rejetée" };
            self.repo
                .publish_gazette(&GazetteArticle {
                    id: Uuid::new_v4(),
                    guild_id: motion.guild_id.clone(),
                    headline: format!("Motion du salon {verdict} : {}", motion.titre),
                    body: format!("Vote final : {for_votes} pour, {against_votes} contre."),
                    published_at: now,
                })
                .await?;
            closed += 1;
        }
        Ok(closed)
    }
}

#[cfg(test)]
#[path = "tests/grand_salon_service.rs"]
mod tests;
