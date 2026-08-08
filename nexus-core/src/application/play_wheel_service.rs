//! Implementation du use case Roue du Destin.
//!
//! Flow (repris de l'ancien `manage_wheel_service` Sentinel, simplifie) :
//!   1. Claim atomique du tirage du jour (`try_claim_today`) — seule la
//!      premiere requete concurrente du jour obtient `true`.
//!   2. Spin RNG (entropie OS en prod).
//!   3. Wallet : credit si payout > 0, debit CLAMPE AU SOLDE si payout < 0
//!      (un joueur ne passe jamais en negatif), rien si payout = 0.
//!   4. Journalisation du spin.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::application::wallet_service::get_or_create_wallet;
use crate::domain::entities::wallet::Wallet;
use crate::domain::entities::wallet::WalletMutation;
use crate::domain::entities::wheel::default_cases;
use crate::domain::entities::wheel::is_memorable_case;
use crate::domain::entities::wheel::spin_cases_with_rng;
use crate::domain::entities::wheel::WheelSpin;
use crate::domain::errors::DomainError;
use crate::ports::inbound::get_wallet::GetWalletUseCase;
use crate::ports::inbound::play_wheel::PlayWheelCommand;
use crate::ports::inbound::play_wheel::PlayWheelResult;
use crate::ports::inbound::play_wheel::PlayWheelUseCase;
use crate::ports::outbound::wallet_repository::WalletRepository;
use crate::ports::outbound::wheel_repository::WheelRepository;

pub struct PlayWheelService {
    wheel_repo: Arc<dyn WheelRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
    config_repo:
        Arc<dyn crate::ports::outbound::system::bot_config_repository::BotConfigRepository>,
}

impl PlayWheelService {
    pub fn new(
        wheel_repo: Arc<dyn WheelRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
        config_repo: Arc<
            dyn crate::ports::outbound::system::bot_config_repository::BotConfigRepository,
        >,
    ) -> Self {
        Self {
            wheel_repo,
            wallet_repo,
            config_repo,
        }
    }
}

#[async_trait]
impl PlayWheelUseCase for PlayWheelService {
    async fn spin(&self, cmd: PlayWheelCommand) -> Result<PlayWheelResult, DomainError> {
        let cfg =
            crate::application::economy_config::load_economy(&self.config_repo, &cmd.guild_id)
                .await?;

        if !cfg.enabled || !cfg.wheel_enabled {
            return Err(DomainError::Validation(
                "La Roue du Destin est desactivee sur ce serveur.".into(),
            ));
        }

        // 1. Verification of cooldown BEFORE spinning (read-only)
        if self
            .wheel_repo
            .has_claimed_recently(&cmd.guild_id, &cmd.user_id, cfg.wheel_cooldown_hours)
            .await?
        {
            // Le message suit le delai reel : annoncer « aujourd'hui » alors
            // que le delai est de six heures serait faux.
            return Err(DomainError::Validation(if cfg.wheel_cooldown_hours >= 24 {
                "Tu as deja tire la Roue du Destin aujourd'hui.".into()
            } else {
                format!(
                    "Tu as deja tire la Roue. Reviens dans moins de {} h.",
                    cfg.wheel_cooldown_hours
                )
            }));
        }

        // 2. Spin RNG (entropie OS) sur les cases DU SERVEUR.
        let cases = {
            let personnalisees = self.wheel_repo.list_cases(&cmd.guild_id).await?;
            if personnalisees.is_empty() {
                default_cases()
            } else {
                personnalisees
            }
        };
        let outcome = {
            let mut rng = rand::thread_rng();
            spin_cases_with_rng(&cases, &mut rng).map_err(|e| {
                DomainError::Validation(format!("La roue de ce serveur est mal reglee : {e}"))
            })?
        };
        let payout = cfg.apply_payout(outcome.case.payout);

        // 3. Wallet : regles pures de credit/debit en memoire.
        let mut wallet = get_or_create_wallet(
            self.wallet_repo.as_ref(),
            &cmd.guild_id,
            &cmd.user_id,
            &cmd.username,
        )
        .await?;
        wallet.username = cmd.username.clone();

        let mutation = if payout > 0 {
            wallet.credit(payout)?;
            Some(WalletMutation {
                amount: payout,
                balance_after: wallet.coins,
                source: "wheel_payout".into(),
                description: format!("Roue du Destin : {}", outcome.case.label),
                reason: None,
            })
        } else if payout < 0 {
            let actual = wallet.debit_clamped(-payout)?;
            if actual > 0 {
                Some(WalletMutation {
                    amount: -actual,
                    balance_after: wallet.coins,
                    source: "wheel_loss".into(),
                    description: format!("Roue du Destin : {}", outcome.case.label),
                    reason: None,
                })
            } else {
                None
            }
        } else {
            None
        };

        let spin = WheelSpin {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            user_id: cmd.user_id.clone(),
            username: cmd.username.clone(),
            case_key: outcome.case.key.to_string(),
            case_label: outcome.case.label.to_string(),
            payout,
            created_at: Utc::now(),
        };

        // 4. Executer la transaction globale
        let claimed = self
            .wheel_repo
            .execute_spin_transaction(
                &cmd.guild_id,
                &cmd.user_id,
                cfg.wheel_cooldown_hours,
                &spin,
                &wallet,
                mutation.as_ref(),
            )
            .await?;

        if !claimed {
            return Err(DomainError::Validation(if cfg.wheel_cooldown_hours >= 24 {
                "Tu as deja tire la Roue du Destin aujourd'hui.".into()
            } else {
                format!(
                    "Tu as deja tire la Roue. Reviens dans moins de {} h.",
                    cfg.wheel_cooldown_hours
                )
            }));
        }

        Ok(PlayWheelResult {
            is_memorable: is_memorable_case(&spin.case_key),
            balance_after: wallet.coins,
            spin,
        })
    }

    async fn can_spin(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError> {
        let cfg =
            crate::application::economy_config::load_economy(&self.config_repo, guild_id).await?;
        if !cfg.enabled || !cfg.wheel_enabled {
            return Ok(false);
        }
        Ok(!self
            .wheel_repo
            .has_claimed_recently(guild_id, user_id, cfg.wheel_cooldown_hours)
            .await?)
    }
}

#[async_trait]
impl GetWalletUseCase for PlayWheelService {
    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Wallet, DomainError> {
        get_or_create_wallet(self.wallet_repo.as_ref(), guild_id, user_id, "").await
    }
}

#[cfg(test)]
#[path = "tests/play_wheel.rs"]
mod tests;
