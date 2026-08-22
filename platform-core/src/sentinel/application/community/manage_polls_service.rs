//! Sondages communautaires.
//!
//! Les regles portees ici : un sondage a au moins deux choix distincts, il se
//! ferme a une date future, et on ne vote pas sur un sondage clos.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::poll::{
    Poll, UpsertPollCommand, MAX_OPTIONS, MIN_OPTIONS,
};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_polls::{ManagePollsUseCase, PollWithVote};
use crate::sentinel::ports::outbound::community::poll_repository::PollRepository;

const MAX_QUESTION_CHARS: usize = 200;
const MAX_LABEL_CHARS: usize = 120;
const MAX_DESCRIPTION_CHARS: usize = 500;
const MAX_LIMIT: i64 = 50;

pub struct ManagePollsService {
    repo: Arc<dyn PollRepository>,
}

impl ManagePollsService {
    pub fn new(repo: Arc<dyn PollRepository>) -> Self {
        Self { repo }
    }

    fn sanitize(mut cmd: UpsertPollCommand) -> Result<UpsertPollCommand, DomainError> {
        cmd.question = cmd.question.trim().to_string();
        if cmd.question.is_empty() {
            return Err(DomainError::ValidationError("question obligatoire".into()));
        }
        if cmd.question.chars().count() > MAX_QUESTION_CHARS {
            return Err(DomainError::ValidationError(
                "question limitee a 200 caracteres".into(),
            ));
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

        // Un sondage deja clos a sa creation ne recueillerait aucune voix.
        if cmd.closes_at <= Utc::now() {
            return Err(DomainError::ValidationError(
                "la date de cloture doit etre dans le futur".into(),
            ));
        }

        // Options : on nettoie d'abord, on compte ensuite. Une option vide
        // saisie par erreur ne doit pas compter comme un choix.
        let mut vues: Vec<String> = Vec::new();
        let mut options = Vec::new();
        for (label, couleur) in std::mem::take(&mut cmd.options) {
            let label = label
                .trim()
                .chars()
                .take(MAX_LABEL_CHARS)
                .collect::<String>();
            if label.is_empty() {
                continue;
            }
            // Deux options identiques rendraient le resultat inexploitable :
            // on ne saurait pas laquelle a ete choisie.
            let cle = label.to_lowercase();
            if vues.contains(&cle) {
                return Err(DomainError::ValidationError(format!(
                    "l'option « {label} » apparait deux fois"
                )));
            }
            vues.push(cle);

            let couleur = couleur
                .map(|c| c.trim().trim_start_matches('#').to_lowercase())
                .filter(|c| c.len() == 6 && c.chars().all(|ch| ch.is_ascii_hexdigit()));

            options.push((label, couleur));
        }

        if options.len() < MIN_OPTIONS {
            return Err(DomainError::ValidationError(
                "il faut au moins deux choix".into(),
            ));
        }
        if options.len() > MAX_OPTIONS {
            return Err(DomainError::ValidationError("dix choix au maximum".into()));
        }
        cmd.options = options;

        Ok(cmd)
    }

    async fn load(&self, id: Uuid) -> Result<Poll, DomainError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("sondage introuvable".into()))
    }
}

#[async_trait]
impl ManagePollsUseCase for ManagePollsService {
    async fn list(
        &self,
        guild_id: &str,
        open_only: bool,
        limit: i64,
    ) -> Result<Vec<Poll>, DomainError> {
        self.repo
            .list(guild_id, open_only, limit.clamp(1, MAX_LIMIT))
            .await
    }

    async fn get(&self, id: Uuid, viewer_id: Option<&str>) -> Result<PollWithVote, DomainError> {
        let poll = self.load(id).await?;
        let my_vote = match viewer_id {
            Some(uid) => self.repo.vote_of(id, uid).await?,
            None => None,
        };
        Ok(PollWithVote { poll, my_vote })
    }

    async fn create(&self, cmd: UpsertPollCommand) -> Result<Poll, DomainError> {
        self.repo.create(&Self::sanitize(cmd)?).await
    }

    async fn close(&self, id: Uuid) -> Result<(), DomainError> {
        if self.repo.set_closed(id, true).await? {
            Ok(())
        } else {
            Err(DomainError::NotFound("sondage introuvable".into()))
        }
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        if self.repo.delete(id).await? {
            Ok(())
        } else {
            Err(DomainError::NotFound("sondage introuvable".into()))
        }
    }

    async fn vote(
        &self,
        poll_id: Uuid,
        option_id: Uuid,
        user_id: &str,
    ) -> Result<Poll, DomainError> {
        let poll = self.load(poll_id).await?;
        if !poll.is_open(Utc::now()) {
            return Err(DomainError::ValidationError("ce sondage est clos".into()));
        }

        // Le repository refuse une option etrangere au sondage : sans ce
        // controle, un client pourrait voter pour l'option d'un autre
        // sondage et fausser deux resultats a la fois.
        if !self.repo.cast_vote(poll_id, option_id, user_id).await? {
            return Err(DomainError::ValidationError(
                "ce choix n'appartient pas a ce sondage".into(),
            ));
        }

        // Relecture : le client affiche les barres a jour, y compris les voix
        // arrivees entre-temps.
        self.load(poll_id).await
    }
}

