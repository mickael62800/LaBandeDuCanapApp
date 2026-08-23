//! Service applicatif du copilote de moderation.
//!
//! Orchestre (lecture seule) : strikes actifs + config d'escalade (reutilise le
//! use case `ManageStrikesUseCase`, source de verite du ladder), historique de
//! sanctions et jurisprudence (port `ModerationCopilotRepository`). Delegue le
//! calcul de la suggestion au service de domaine PUR `suggest_sanction`.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use chrono::Utc;

use crate::sentinel::domain::entities::moderation::copilot::MemberModerationContext;
use crate::sentinel::domain::entities::moderation::copilot::PrecedentDistribution;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::domain::services::moderation::moderation_copilot::suggest_sanction;
use crate::sentinel::domain::services::moderation::moderation_copilot::SuggestInputs;
use crate::sentinel::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase;
use crate::sentinel::ports::inbound::moderation::moderation_copilot::ModerationCopilotUseCase;
use crate::sentinel::ports::outbound::moderation::moderation_copilot_repository::ModerationCopilotRepository;

/// Bornes de securite sur les parametres de requete.
const MAX_LOOKBACK_DAYS: i64 = 365;
const MIN_LOOKBACK_DAYS: i64 = 1;
const MAX_MIN_PRECEDENTS: u32 = 100;

/// Defauts appliques quand le parametre est absent (0 = non renseigne, cas des
/// clients gRPC dont le proto met 0 par defaut). Alignes sur les defauts du
/// handler HTTP pour que tout client obtienne la meme fenetre intentionnelle.
const DEFAULT_LOOKBACK_DAYS: i64 = 90;
const DEFAULT_MIN_PRECEDENTS: u32 = 3;

pub struct ManageModerationCopilotService {
    strikes_uc: Arc<dyn ManageStrikesUseCase>,
    repo: Arc<dyn ModerationCopilotRepository>,
}

impl ManageModerationCopilotService {
    pub fn new(
        strikes_uc: Arc<dyn ManageStrikesUseCase>,
        repo: Arc<dyn ModerationCopilotRepository>,
    ) -> Self {
        Self { strikes_uc, repo }
    }
}

#[async_trait]
impl ModerationCopilotUseCase for ManageModerationCopilotService {
    async fn get_member_context(
        &self,
        guild_id: &str,
        user_id: &str,
        lookback_days: i64,
        min_precedents: u32,
    ) -> Result<MemberModerationContext, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(guild_id)?;
        crate::sentinel::application::validation::validate_non_empty(user_id, "user_id")?;

        // 0 = parametre non renseigne (defaut proto gRPC) -> applique le defaut
        // intentionnel AVANT de clamper, sinon un appel gRPC nu tomberait sur
        // 1/1 au lieu de 90/3.
        let lookback_days = if lookback_days == 0 {
            DEFAULT_LOOKBACK_DAYS
        } else {
            lookback_days
        };
        let min_precedents = if min_precedents == 0 {
            DEFAULT_MIN_PRECEDENTS
        } else {
            min_precedents
        };
        let lookback_days = lookback_days.clamp(MIN_LOOKBACK_DAYS, MAX_LOOKBACK_DAYS);
        let min_precedents = min_precedents.clamp(1, MAX_MIN_PRECEDENTS);
        let since = Utc::now() - Duration::days(lookback_days);

        // 1. Strikes actifs + echelle d'escalade (reutilise le use case strikes).
        let active_strikes = self
            .strikes_uc
            .get_active_strikes(guild_id, user_id)
            .await?
            .len() as u32;
        let config = self.strikes_uc.get_config(guild_id).await?;

        // 2. Historique de sanctions.
        let sanctions_by_type = self
            .repo
            .count_sanctions_by_type(guild_id, user_id, since)
            .await?;
        let last_sanction_at = self.repo.last_sanction_at(guild_id, user_id).await?;
        let open_reviews = self.repo.count_open_reviews(guild_id, user_id).await?;

        // 3. Jurisprudence : categorie dominante puis distribution (hors voting).
        let precedents = match self
            .repo
            .dominant_flag_category(guild_id, user_id, since)
            .await?
        {
            Some(category) => {
                self.repo
                    .aggregate_decided_by_flag(guild_id, &category, since)
                    .await?
            }
            None => PrecedentDistribution::default(),
        };

        // 4. Suggestion (domaine pur).
        let suggestion = suggest_sanction(&SuggestInputs {
            active_strikes,
            thresholds: &config.thresholds,
            precedents: &precedents,
            min_precedents,
        });

        Ok(MemberModerationContext {
            active_strikes,
            sanctions_by_type,
            last_sanction_at,
            open_reviews,
            precedents,
            suggestion,
        })
    }
}
