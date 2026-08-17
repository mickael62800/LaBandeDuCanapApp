//! Fouille sous les coussins, avec fenetre de defense.
//!
//! Le vol se jouait a pile ou face : un pourcentage fixe, sans que la cible
//! puisse quoi que ce soit. Perdre sept fois sur dix sans avoir eu son mot a
//! dire n'est pas un jeu, c'est une taxe.
//!
//! Le modele est celui de l'ancien Coup de Coude. `open` ouvre la fouille et
//! ne deplace RIEN : la victime a quelques dizaines de secondes pour serrer
//! les coussins. Si elle reagit (`defend`), elle garde toute sa defense ; si
//! elle laisse passer, le job (`resolve_expired`) tranche avec le malus
//! d'absence, et le voleur passe beaucoup plus facilement.
//!
//! La regle du jet vit dans `domain::entities::coussin_steal` ; ici on ne fait
//! que l'alimenter et en tirer les consequences monetaires.
//!
//! Toutes les valeurs d'equilibre viennent de la configuration du serveur.

use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;

use crate::nexus::application::economy_config::{load_coussin, CoussinConfig};
use crate::nexus::domain::entities::coussin::PlayerClass;
use crate::nexus::domain::entities::coussin_steal::{
    resolve_steal, Defense, StealRoll, STEAL_DICE_FACES,
};
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::{
    inbound::coussin_steal::{CoussinStealUseCase, OpenedSteal, StealOutcome},
    outbound::{
        coussin_repository::CoussinRepository,
        coussin_steal_repository::{CoussinStealRepository, StealAttempt},
        system::bot_config_repository::BotConfigRepository,
    },
};

pub struct CoussinStealService {
    repo: Arc<dyn CoussinStealRepository>,
    /// Profils : la defense de la victime pese sur le jet.
    profiles: Arc<dyn CoussinRepository>,
    config_repo: Arc<dyn BotConfigRepository>,
}

impl CoussinStealService {
    pub fn new(
        repo: Arc<dyn CoussinStealRepository>,
        profiles: Arc<dyn CoussinRepository>,
        config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            repo,
            profiles,
            config_repo,
        }
    }

    /// Configuration du serveur, apres verification que le jeu et la fouille
    /// sont ouverts.
    async fn open_config(&self, guild_id: &str) -> Result<CoussinConfig, DomainError> {
        let cfg = load_coussin(&self.config_repo, guild_id).await?;
        cfg.ensure_enabled()?;
        if !cfg.steal_enabled {
            return Err(DomainError::Validation(
                "les vols sont desactives sur ce serveur".into(),
            ));
        }
        Ok(cfg)
    }

    /// Joue le jet et applique le resultat sur les portefeuilles.
    ///
    /// La reclamation de la tentative a deja eu lieu : cette methode n'est
    /// atteinte qu'une fois, quel que soit le vainqueur de la course entre la
    /// victime et le job.
    async fn settle(
        &self,
        attempt: StealAttempt,
        defense: Defense,
    ) -> Result<StealOutcome, DomainError> {
        let cfg = load_coussin(&self.config_repo, &attempt.guild_id).await?;

        // Sans controle de delai : la fenetre de defense a pu laisser le temps
        // au voleur d'aller fouiller ailleurs. Le refuser ici laisserait cette
        // fouille-ci sans denouement.
        let (thief_coins, victim_coins) = self
            .repo
            .settlement_balances(&attempt.guild_id, &attempt.thief_id, &attempt.victim_id)
            .await?;

        let thief = self
            .profiles
            .find_profile(&attempt.guild_id, &attempt.thief_id)
            .await?;
        let victim = self
            .profiles
            .find_profile(&attempt.guild_id, &attempt.victim_id)
            .await?;

        let is_piegeur = thief
            .as_ref()
            .map(|p| p.class == PlayerClass::Piegeur)
            .unwrap_or(false);
        let victim_def = victim.as_ref().map(|p| p.def).unwrap_or(0);

        let roll = self.roll(is_piegeur, victim_def, defense, cfg.steal_absence_malus);

        let amount = if roll.success {
            cfg.steal_gain(victim_coins)
        } else {
            cfg.steal_penalty(thief_coins)
        };

        self.repo
            .transfer(
                &attempt.guild_id,
                &attempt.thief_id,
                &attempt.victim_id,
                amount,
                roll.success,
                cfg.steal_cooldown_minutes,
            )
            .await?;

        let defended = defense == Defense::Reacted;
        self.repo
            .record_outcome(attempt.id, defended, roll.success, amount)
            .await?;

        Ok(StealOutcome {
            attempt_id: attempt.id,
            guild_id: attempt.guild_id,
            thief_id: attempt.thief_id,
            victim_id: attempt.victim_id,
            channel_id: attempt.channel_id,
            message_id: attempt.message_id,
            defended,
            success: roll.success,
            amount,
            thief_total: roll.thief_total,
            victim_total: roll.victim_total,
            absence_malus: roll.absence_malus,
        })
    }

    /// Les deux des, puis la regle du domaine. Isole pour que le tirage reste
    /// le seul endroit non deterministe de la resolution.
    fn roll(
        &self,
        is_piegeur: bool,
        victim_def: i32,
        defense: Defense,
        absence_malus: i32,
    ) -> StealRoll {
        let mut rng = rand::thread_rng();
        let thief_die = rng.gen_range(1..=STEAL_DICE_FACES);
        let victim_die = rng.gen_range(1..=STEAL_DICE_FACES);
        resolve_steal(
            thief_die,
            victim_die,
            is_piegeur,
            victim_def,
            defense,
            absence_malus,
        )
    }
}

#[async_trait]
impl CoussinStealUseCase for CoussinStealService {
    async fn open(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        channel_id: &str,
    ) -> Result<OpenedSteal, DomainError> {
        if thief_id == victim_id {
            return Err(DomainError::Validation(
                "impossible de se voler soi-meme".into(),
            ));
        }

        let cfg = self.open_config(guild_id).await?;

        let (_, victim_coins) = self.repo.balances(guild_id, thief_id, victim_id).await?;

        // Plancher de pauvrete : sans lui, on peut achever quelqu'un qui n'a
        // deja plus rien. Ca ne rapporte presque rien et ca degoute.
        if victim_coins < cfg.steal_min_victim_coins {
            return Err(DomainError::Validation(format!(
                "cible trop pauvre (moins de {} coins)",
                cfg.steal_min_victim_coins
            )));
        }

        let attempt = self
            .repo
            .open_attempt(
                guild_id,
                thief_id,
                victim_id,
                channel_id,
                cfg.steal_defense_window_seconds,
            )
            .await?;

        Ok(OpenedSteal {
            attempt_id: attempt.id,
            victim_id: attempt.victim_id,
            expires_at: attempt.expires_at,
            defense_window_seconds: cfg.steal_defense_window_seconds,
        })
    }

    async fn attach_message(
        &self,
        attempt_id: uuid::Uuid,
        message_id: &str,
    ) -> Result<(), DomainError> {
        self.repo.attach_message(attempt_id, message_id).await
    }

    async fn defend(
        &self,
        attempt_id: uuid::Uuid,
        victim_id: &str,
    ) -> Result<StealOutcome, DomainError> {
        // Reclamation atomique : si le job a resolu la fouille dans la meme
        // seconde, la victime apprend qu'elle a reagi trop tard plutot que de
        // voir le vol se jouer deux fois.
        let attempt = self
            .repo
            .claim_attempt(attempt_id, Some(victim_id))
            .await?
            .ok_or_else(|| {
                DomainError::Validation(
                    "Trop tard : les coussins ont deja ete fouilles.".to_string(),
                )
            })?;

        self.settle(attempt, Defense::Reacted).await
    }

    async fn resolve_expired(&self, limit: i64) -> Result<Vec<StealOutcome>, DomainError> {
        let attempts = self
            .repo
            .claim_expired_attempts(limit.clamp(1, 200))
            .await?;
        let mut out = Vec::with_capacity(attempts.len());
        for attempt in attempts {
            let id = attempt.id;
            // Une fouille qui echoue a se regler ne doit pas empecher les
            // suivantes : elle est deja reclamee, donc jamais rejouee.
            match self.settle(attempt, Defense::Absent).await {
                Ok(outcome) => out.push(outcome),
                Err(error) => {
                    tracing::warn!(%error, attempt_id = %id, "fouille expiree non reglee");
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "tests/coussin_steal_service.rs"]
mod tests;
