use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Datelike;
use chrono::Utc;

use crate::sentinel::domain::entities::community::level::level_from_xp;
use crate::sentinel::domain::entities::community::level::UserLevel;
use crate::sentinel::domain::entities::community::level::XpSource;
use crate::sentinel::domain::entities::community::progression_calc as calc;
use crate::sentinel::domain::entities::community::progression_calc::StreakState;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_levels::AddXpCommand;
use crate::sentinel::ports::inbound::community::manage_levels::AddXpResult;
use crate::sentinel::ports::inbound::community::manage_levels::ManageLevelsUseCase;
use crate::sentinel::ports::inbound::community::manage_levels::RecordActivityResult;
use crate::sentinel::ports::inbound::community::manage_levels::RecordTextActivityCommand;
use crate::sentinel::ports::inbound::community::manage_levels::RecordVoiceActivityCommand;
use crate::sentinel::ports::inbound::community::manage_levels::ResetTarget;
use crate::sentinel::ports::inbound::community::manage_levels::SetUserXpCommand;
use crate::sentinel::ports::outbound::community::level_repository::LevelRepository;
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Nom du bot sous lequel la config progression est stockee en DB.
use crate::sentinel::domain::entities::system::bot_names::PROGRESSION_BOT as PROGRESSION_BOT_NAME;

pub struct ManageLevelsService {
    repo: Arc<dyn LevelRepository>,
    bot_config: Arc<dyn BotConfigRepository>,
    /// Cooldown anti-farm XP texte, server-side (remplace le `XpCooldown`
    /// in-memory du bot). Cle = (guild_id, user_id).
    text_cooldown: Mutex<HashMap<(String, String), Instant>>,
}

impl ManageLevelsService {
    pub fn new(repo: Arc<dyn LevelRepository>, bot_config: Arc<dyn BotConfigRepository>) -> Self {
        Self {
            repo,
            bot_config,
            text_cooldown: Mutex::new(HashMap::new()),
        }
    }

    /// Charge la config progression du serveur sous forme de map cle->valeur.
    async fn load_config(&self, guild_id: &str) -> HashMap<String, String> {
        self.bot_config
            .get_config(guild_id, PROGRESSION_BOT_NAME)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|c| (c.config_key, c.config_value))
            .collect()
    }

    /// Reserve ATOMIQUEMENT le gain d'XP texte : renvoie `true` (et pose le
    /// timestamp) si le cooldown est expire. Reproduit `XpCooldown::try_claim`.
    fn try_claim_text(&self, guild_id: &str, user_id: &str, cooldown_secs: u64) -> bool {
        let now = Instant::now();
        let key = (guild_id.to_string(), user_id.to_string());
        let mut map = self.text_cooldown.lock().unwrap();
        if cooldown_secs == 0 {
            map.insert(key, now);
            return true;
        }
        let cooldown = Duration::from_secs(cooldown_secs);
        let allowed = match map.get(&key) {
            Some(last) => now.duration_since(*last) >= cooldown,
            None => true,
        };
        if allowed {
            map.insert(key, now);
            // Nettoyage best-effort des entrees expirees pour borner la memoire.
            if map.len() > 10_000 {
                map.retain(|_, last| now.duration_since(*last) < Duration::from_secs(300));
            }
        }
        allowed
    }
}

fn config_u64(map: &HashMap<String, String>, key: &str, default: u64) -> u64 {
    map.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn config_f64(map: &HashMap<String, String>, key: &str, default: f64) -> f64 {
    map.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn config_bool(map: &HashMap<String, String>, key: &str, default: bool) -> bool {
    map.get(key).map(|v| v == "true").unwrap_or(default)
}

fn config_str<'a>(map: &'a HashMap<String, String>, key: &str) -> &'a str {
    map.get(key).map(|s| s.as_str()).unwrap_or("")
}

#[async_trait]
impl ManageLevelsUseCase for ManageLevelsService {
    async fn add_xp(&self, cmd: AddXpCommand) -> Result<AddXpResult, DomainError> {
        // Validation
        crate::sentinel::application::validation::validate_positive(cmd.amount, "Le montant XP")?;
        if cmd.amount > 10000 {
            return Err(DomainError::ValidationError(
                "Le montant XP ne peut pas depasser 10000".into(),
            ));
        }

        // UPDATE atomique. RETURNING retourne les levels PRE-update (le SQL
        // ne modifie pas les colonnes level_*), ce qui elimine la race condition
        // entre lecture de l'ancien etat et l'update.
        let user_level_pre = self
            .repo
            .add_xp_atomic(
                &cmd.guild_id,
                &cmd.user_id,
                &cmd.username,
                cmd.amount,
                cmd.source,
            )
            .await?;

        // Anciens niveaux = ceux retournes par RETURNING (non touches par l'UPDATE).
        let old_level_text = user_level_pre.level_text;
        let old_level_voice = user_level_pre.level_voice;
        let old_level_global = user_level_pre.level;

        // Recalculer les niveaux depuis le nouvel XP.
        let mut user_level = user_level_pre;
        user_level.level = level_from_xp(user_level.xp);
        user_level.level_text = level_from_xp(user_level.xp_text);
        user_level.level_voice = level_from_xp(user_level.xp_voice);

        // Persister les niveaux recalcules.
        if let Err(e) = self.repo.upsert_user_level(&user_level).await {
            tracing::error!(
                error = %e,
                guild_id = %cmd.guild_id,
                user_id = %cmd.user_id,
                xp = user_level.xp,
                level = user_level.level,
                "Echec mise a jour niveaux apres ajout XP"
            );
            return Err(e);
        }

        // Detecter le level-up de la source specifique
        let (old_source_level, new_source_level) = match cmd.source {
            XpSource::Text => (old_level_text, user_level.level_text),
            XpSource::Voice => (old_level_voice, user_level.level_voice),
        };

        let leveled_up = new_source_level > old_source_level;

        Ok(AddXpResult {
            user_level,
            leveled_up,
            old_level: old_source_level,
            old_level_global,
            source: cmd.source,
        })
    }

    async fn record_text_activity(
        &self,
        cmd: RecordTextActivityCommand,
    ) -> Result<RecordActivityResult, DomainError> {
        let guild_id = cmd.guild_id.as_str().to_string();
        let user_id = cmd.user_id.as_str().to_string();
        let config = self.load_config(&guild_id).await;

        if !config_bool(&config, "enabled", false) {
            return Ok(skipped_result(
                &guild_id,
                &user_id,
                &cmd.username,
                XpSource::Text,
            ));
        }

        // Cooldown anti-farm (texte uniquement, comme l'ancien bot).
        let cooldown_secs = config_u64(&config, "xp_cooldown_secs", 60);
        if !self.try_claim_text(&guild_id, &user_id, cooldown_secs) {
            return Ok(skipped_result(
                &guild_id,
                &user_id,
                &cmd.username,
                XpSource::Text,
            ));
        }

        // Streak server-side (depuis l'etat persiste).
        let now = Utc::now();
        let today_day = now.ordinal();
        let today_year = now.year();
        let streak_enabled = config_bool(&config, "streak_enabled", true);
        let (streak_mult, streak_outcome, streak_current) = if streak_enabled {
            let state = self
                .repo
                .get_streak(&guild_id, &user_id)
                .await?
                .unwrap_or_default();
            let bonus = config_f64(
                &config,
                "streak_bonus_per_week",
                calc::DEFAULT_STREAK_BONUS_PER_WEEK,
            );
            let max_mult = config_f64(
                &config,
                "streak_max_multiplier",
                calc::DEFAULT_STREAK_MAX_MULTIPLIER,
            );
            let outcome = calc::compute_streak(state, today_day, today_year, bonus, max_mult);
            (outcome.multiplier, Some(outcome), outcome.current)
        } else {
            (1.0, None, 0)
        };

        // Multiplicateurs channel/role.
        let channel_mults = calc::parse_multipliers(config_str(&config, "xp_channel_multipliers"));
        let role_mults = calc::parse_multipliers(config_str(&config, "xp_role_multipliers"));
        let channel_mult = calc::get_channel_multiplier(&channel_mults, cmd.channel_id);
        let role_mult = calc::get_role_multiplier(&role_mults, &cmd.role_ids);

        // Montant : base x channel x role x streak, clamp [1, 1000].
        let base_xp = config_u64(&config, "xp_per_message", 15) as f64;
        let amount =
            calc::calc_xp_amount(base_xp, channel_mult, role_mult, streak_mult, 1.0, 1000.0);

        let add = self
            .add_xp(AddXpCommand {
                guild_id: cmd.guild_id,
                user_id: cmd.user_id,
                username: cmd.username,
                amount,
                source: XpSource::Text,
            })
            .await?;

        // Persister le streak seulement si nouveau jour.
        if let Some(outcome) = streak_outcome {
            if outcome.new_day {
                self.repo
                    .update_streak(
                        &guild_id,
                        &user_id,
                        StreakState {
                            current: outcome.current,
                            best: outcome.best,
                            last_day: today_day,
                            last_year: today_year,
                        },
                    )
                    .await?;
            }
        }

        Ok(RecordActivityResult {
            skipped: false,
            xp_gained: amount,
            user_level: add.user_level,
            leveled_up: add.leveled_up,
            old_level: add.old_level,
            old_level_global: add.old_level_global,
            source: add.source,
            streak_current,
        })
    }

    async fn record_voice_activity(
        &self,
        cmd: RecordVoiceActivityCommand,
    ) -> Result<RecordActivityResult, DomainError> {
        let guild_id = cmd.guild_id.as_str().to_string();
        let user_id = cmd.user_id.as_str().to_string();
        let config = self.load_config(&guild_id).await;

        if !config_bool(&config, "enabled", false) {
            return Ok(skipped_result(
                &guild_id,
                &user_id,
                &cmd.username,
                XpSource::Voice,
            ));
        }

        let xp_per_minute = config_u64(&config, "xp_per_voice_minute", 5) as f64;
        let channel_mults = calc::parse_multipliers(config_str(&config, "xp_channel_multipliers"));
        let role_mults = calc::parse_multipliers(config_str(&config, "xp_role_multipliers"));
        let channel_mult = calc::get_channel_multiplier(&channel_mults, cmd.channel_id);
        let role_mult = calc::get_role_multiplier(&role_mults, &cmd.role_ids);

        // Base = (secondes / 60) x xp/min. Pas de streak sur le vocal.
        // Clamp [0, 10000] : au-dela l'ancien bot perdait tout l'XP (add_xp
        // rejette > 10000).
        let base_voice = (cmd.seconds as f64 / 60.0) * xp_per_minute;
        let amount = calc::calc_xp_amount(base_voice, channel_mult, role_mult, 1.0, 0.0, 10_000.0);
        if amount <= 0 {
            return Ok(skipped_result(
                &guild_id,
                &user_id,
                &cmd.username,
                XpSource::Voice,
            ));
        }

        let add = self
            .add_xp(AddXpCommand {
                guild_id: cmd.guild_id,
                user_id: cmd.user_id,
                username: cmd.username,
                amount,
                source: XpSource::Voice,
            })
            .await?;

        Ok(RecordActivityResult {
            skipped: false,
            xp_gained: amount,
            user_level: add.user_level,
            leveled_up: add.leveled_up,
            old_level: add.old_level,
            old_level_global: add.old_level_global,
            source: add.source,
            streak_current: 0,
        })
    }

    async fn get_user_level(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UserLevel, DomainError> {
        self.repo
            .get_user_level(guild_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Niveau introuvable pour {user_id}")))
    }

    async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError> {
        self.repo.get_leaderboard(guild_id, limit).await
    }

    async fn get_leaderboard_by_source(
        &self,
        guild_id: &str,
        source: XpSource,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError> {
        self.repo
            .get_leaderboard_by_source(guild_id, source, limit)
            .await
    }

    async fn set_user_xp(&self, cmd: SetUserXpCommand) -> Result<UserLevel, DomainError> {
        let mut user = self
            .repo
            .get_user_level(cmd.guild_id.as_ref(), cmd.user_id.as_ref())
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!(
                    "User {} n'a pas encore de progression sur la guild {}",
                    cmd.user_id.as_ref(),
                    cmd.guild_id.as_ref()
                ))
            })?;

        // Borne haute (anti-overflow) : bien au-dela de l'XP du niveau max, mais
        // loin de i64::MAX pour que xp_text + xp_voice ne deborde jamais.
        const MAX_XP: i64 = 1_000_000_000_000_000; // 1e15
        if let Some(xp_t) = cmd.xp_text {
            if !(0..=MAX_XP).contains(&xp_t) {
                return Err(DomainError::ValidationError(
                    "xp_text hors bornes (0..=1e15)".into(),
                ));
            }
            user.xp_text = xp_t;
            user.level_text = level_from_xp(xp_t);
        }
        if let Some(xp_v) = cmd.xp_voice {
            if !(0..=MAX_XP).contains(&xp_v) {
                return Err(DomainError::ValidationError(
                    "xp_voice hors bornes (0..=1e15)".into(),
                ));
            }
            user.xp_voice = xp_v;
            user.level_voice = level_from_xp(xp_v);
        }
        user.xp = user.xp_text.saturating_add(user.xp_voice);
        user.level = level_from_xp(user.xp);
        user.updated_at = Utc::now();

        self.repo.upsert_user_level(&user).await?;
        let _ = self.repo.refresh_leaderboard_view().await;
        Ok(user)
    }

    async fn reset_user_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        target: ResetTarget,
    ) -> Result<UserLevel, DomainError> {
        let mut user = self
            .repo
            .get_user_level(guild_id, user_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!(
                    "User {user_id} n'a pas encore de progression sur la guild {guild_id}"
                ))
            })?;

        match target {
            ResetTarget::Text => {
                user.xp_text = 0;
                user.level_text = 0;
            }
            ResetTarget::Voice => {
                user.xp_voice = 0;
                user.level_voice = 0;
            }
            ResetTarget::All => {
                user.xp_text = 0;
                user.level_text = 0;
                user.xp_voice = 0;
                user.level_voice = 0;
            }
        }
        user.xp = user.xp_text + user.xp_voice;
        user.level = level_from_xp(user.xp);
        user.updated_at = Utc::now();

        self.repo.upsert_user_level(&user).await?;
        let _ = self.repo.refresh_leaderboard_view().await;
        Ok(user)
    }
}

/// Construit un `RecordActivityResult` "sans effet" (cooldown, module
/// desactive, montant nul) : le bot ne doit rien afficher.
fn skipped_result(
    guild_id: &str,
    user_id: &str,
    username: &str,
    source: XpSource,
) -> RecordActivityResult {
    let now = Utc::now();
    RecordActivityResult {
        skipped: true,
        xp_gained: 0,
        user_level: UserLevel {
            id: uuid::Uuid::nil(),
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            username: username.to_string(),
            xp: 0,
            level: 0,
            xp_text: 0,
            level_text: 0,
            xp_voice: 0,
            level_voice: 0,
            last_xp_at: now,
            created_at: now,
            updated_at: now,
        },
        leveled_up: false,
        old_level: 0,
        old_level_global: 0,
        source,
        streak_current: 0,
    }
}

#[cfg(test)]
#[path = "tests/manage_levels.rs"]
mod tests;
