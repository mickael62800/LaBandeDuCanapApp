//! Recherche de joueurs.
//!
//! Le service porte deux regles que ni la base ni le transport ne peuvent
//! tenir : la normalisation des champs libres, et le fait qu'une annonce
//! n'appartient qu'a son auteur — le staff excepte.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::lfg::{LfgPost, UpsertLfgCommand};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_lfg::ManageLfgUseCase;
use crate::sentinel::ports::outbound::community::lfg_repository::LfgRepository;

const MAX_GAME_CHARS: usize = 80;
const MAX_WHEN_CHARS: usize = 80;
const MAX_DESCRIPTION_CHARS: usize = 500;
const MAX_SLOTS: i32 = 50;

/// Plafond de liste. Au-dela, la section devient illisible et la page lourde.
const MAX_LIMIT: i64 = 100;

pub struct ManageLfgService {
    repo: Arc<dyn LfgRepository>,
}

impl ManageLfgService {
    pub fn new(repo: Arc<dyn LfgRepository>) -> Self {
        Self { repo }
    }

    fn sanitize(mut cmd: UpsertLfgCommand) -> Result<UpsertLfgCommand, DomainError> {
        cmd.game = cmd.game.trim().to_string();
        if cmd.game.is_empty() {
            return Err(DomainError::ValidationError("jeu obligatoire".into()));
        }
        if cmd.game.chars().count() > MAX_GAME_CHARS {
            return Err(DomainError::ValidationError(
                "nom de jeu limite a 80 caracteres".into(),
            ));
        }

        // Un « 0 place » n'a aucun sens, un « 999 » non plus : c'est une
        // erreur de saisie, pas une annonce.
        if !(1..=MAX_SLOTS).contains(&cmd.slots) {
            return Err(DomainError::ValidationError(
                "nombre de joueurs recherches entre 1 et 50".into(),
            ));
        }

        cmd.when_text = cmd.when_text.trim().to_string();
        if cmd.when_text.chars().count() > MAX_WHEN_CHARS {
            return Err(DomainError::ValidationError(
                "creneau limite a 80 caracteres".into(),
            ));
        }
        // Champ facultatif : plutot que de refuser, on donne la formulation
        // qui correspond a une annonce sans horaire.
        if cmd.when_text.is_empty() {
            cmd.when_text = "quand vous voulez".to_string();
        }

        cmd.description = cmd
            .description
            .map(|d| {
                d.trim()
                    .chars()
                    .take(MAX_DESCRIPTION_CHARS)
                    .collect::<String>()
            })
            .filter(|d| !d.is_empty());

        cmd.author_name = cmd.author_name.trim().to_string();

        Ok(cmd)
    }

    /// L'annonce, apres verification que l'acteur a le droit d'y toucher.
    async fn owned_by(
        &self,
        id: Uuid,
        actor_id: &str,
        is_staff: bool,
    ) -> Result<LfgPost, DomainError> {
        let post = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("annonce introuvable".into()))?;

        if !is_staff && post.author_id != actor_id {
            return Err(DomainError::Forbidden(
                "cette annonce n'est pas la tienne".into(),
            ));
        }
        Ok(post)
    }

    /// Relit l'annonce apres modification des interesses.
    ///
    /// Le client a besoin de la liste a jour pour afficher les avatars ; la
    /// recalculer cote front a partir de la reponse partielle divergerait de
    /// la base des qu'un autre membre repond en meme temps.
    async fn reload(&self, id: Uuid) -> Result<LfgPost, DomainError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("annonce introuvable".into()))
    }
}

#[async_trait]
impl ManageLfgUseCase for ManageLfgService {
    async fn list(
        &self,
        guild_id: &str,
        live_only: bool,
        limit: i64,
    ) -> Result<Vec<LfgPost>, DomainError> {
        self.repo
            .list(guild_id, live_only, limit.clamp(1, MAX_LIMIT))
            .await
    }

    async fn get(&self, id: Uuid) -> Result<LfgPost, DomainError> {
        self.reload(id).await
    }

    async fn create(&self, cmd: UpsertLfgCommand) -> Result<LfgPost, DomainError> {
        self.repo.create(&Self::sanitize(cmd)?).await
    }

    async fn close(&self, id: Uuid, actor_id: &str, is_staff: bool) -> Result<(), DomainError> {
        self.owned_by(id, actor_id, is_staff).await?;
        self.repo.set_open(id, false).await?;
        Ok(())
    }

    async fn delete(&self, id: Uuid, actor_id: &str, is_staff: bool) -> Result<(), DomainError> {
        self.owned_by(id, actor_id, is_staff).await?;
        self.repo.delete(id).await?;
        Ok(())
    }

    async fn join(&self, id: Uuid, user_id: &str, username: &str) -> Result<LfgPost, DomainError> {
        let post = self.reload(id).await?;

        // Se manifester sur une annonce close ou expiree ne mene nulle part :
        // l'auteur ne la regarde plus.
        if !post.is_live(Utc::now()) {
            return Err(DomainError::ValidationError(
                "cette annonce n'est plus ouverte".into(),
            ));
        }
        // L'auteur cherche des gens, il ne peut pas se compter lui-meme :
        // sinon « cherche 2 joueurs » afficherait 1 place restante des la
        // creation.
        if post.author_id == user_id {
            return Err(DomainError::ValidationError(
                "tu es deja l'auteur de cette annonce".into(),
            ));
        }

        self.repo.add_interest(id, user_id, username.trim()).await?;
        self.reload(id).await
    }

    async fn leave(&self, id: Uuid, user_id: &str) -> Result<LfgPost, DomainError> {
        self.repo.remove_interest(id, user_id).await?;
        self.reload(id).await
    }
}

