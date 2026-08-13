use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::enums::moderation::flag_type::FlagType;

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub flag_type: FlagType,
    pub weight: f64,
    pub threshold_warn: f64,
    pub threshold_delete: f64,
    pub threshold_mute: f64,
    pub threshold_ban: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Rule {
    pub fn new(guild_id: GuildId, flag_type: FlagType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            guild_id,
            weight: Self::default_weight_for(&flag_type),
            flag_type,
            threshold_warn: 2.0,
            threshold_delete: 4.0,
            threshold_mute: 6.0,
            threshold_ban: 9.0,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Jeu de regles de moderation par defaut seede a l'enregistrement d'une
    /// guild. Valeurs metier (poids + seuils) — la persistance idempotente
    /// (`ON CONFLICT (guild_id, flag_type) DO NOTHING`) vit dans `RuleRepository`.
    pub fn default_seed(guild_id: &GuildId) -> Vec<Rule> {
        // (flag_type, weight, warn, delete, mute, ban)
        const DEFAULTS: [(FlagType, f64, f64, f64, f64, f64); 10] = [
            (FlagType::Spam, 2.0, 2.0, 4.0, 6.0, 9.0),
            (FlagType::Insult, 2.0, 2.0, 4.0, 6.0, 8.0),
            (FlagType::Link, 1.0, 3.0, 5.0, 7.0, 9.0),
            (FlagType::Phishing, 3.5, 1.0, 2.5, 4.0, 6.0),
            (FlagType::Nsfw, 3.0, 1.5, 3.0, 5.0, 8.0),
            (FlagType::Illicit, 3.5, 1.0, 2.5, 4.0, 6.0),
            (FlagType::Threat, 3.5, 1.0, 2.0, 4.0, 6.0),
            (FlagType::Rage, 2.5, 2.0, 3.5, 5.0, 7.0),
            (FlagType::Anger, 2.0, 2.5, 4.0, 6.0, 8.0),
            (FlagType::Harassment, 3.0, 1.5, 2.5, 4.5, 7.0),
        ];
        let now = Utc::now();
        DEFAULTS
            .into_iter()
            .map(|(flag_type, weight, warn, delete, mute, ban)| Rule {
                id: Uuid::new_v4(),
                guild_id: guild_id.clone(),
                flag_type,
                weight,
                threshold_warn: warn,
                threshold_delete: delete,
                threshold_mute: mute,
                threshold_ban: ban,
                enabled: true,
                created_at: now,
                updated_at: now,
            })
            .collect()
    }

    fn default_weight_for(flag_type: &FlagType) -> f64 {
        match flag_type {
            FlagType::Spam => 3.0,
            FlagType::Insult => 5.0,
            FlagType::Profanity => 1.0,
            FlagType::Link => 1.0,
            FlagType::Phishing => 7.0,
            FlagType::Nsfw => 8.0,
            FlagType::Illicit => 9.0,
            FlagType::Anger => 3.0,
            FlagType::Rage => 6.0,
            FlagType::Threat => 8.0,
            FlagType::Harassment => 7.0,
        }
    }
}

#[cfg(test)]
#[path = "tests/rule.rs"]
mod tests;
