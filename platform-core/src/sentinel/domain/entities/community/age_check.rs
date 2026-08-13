//! Decision de la verification d'age au reglement.
//!
//! Regle metier PURE : a partir de l'age declare et de la config serveur
//! (`age_minimum`, `age_ban_days_per_year`), on decide si le membre obtient
//! l'acces (`Grant`) ou s'il est banni temporairement (`Ban`) jusqu'a ce qu'il
//! atteigne l'age minimum. La formule de duree du ban vit ici, plus dans le bot.

use chrono::{DateTime, Duration, Utc};

/// Issue de l'evaluation d'une declaration d'age.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgeCheckDecision {
    /// Age suffisant -> acces accorde. Le bot assigne le role membre et
    /// retire le role temporaire (action Discord).
    Grant,
    /// Age insuffisant -> ban temporaire. Le bot bannit sur Discord, programme
    /// le deban (`unban_at`) et enregistre l'age-ban via l'API.
    Ban {
        /// Nombre d'annees de ban (>= 1) : `(age_minimum - declared_age).max(1)`.
        years: i32,
        /// Date/heure du deban automatique programme.
        unban_at: DateTime<Utc>,
        /// Raison du ban, pour l'audit Discord.
        reason: String,
    },
}

/// Regle metier pure de la verification d'age.
///
/// - `declared_age >= age_minimum` -> `Grant`.
/// - sinon -> `Ban` avec `years = (age_minimum - declared_age).max(1)` et
///   `unban_at = now + years * ban_days_per_year jours` (`ban_days_per_year`
///   ramene a >= 1, comme cote bot historique).
pub fn decide_age_check(
    declared_age: i32,
    age_minimum: i32,
    ban_days_per_year: i32,
    now: DateTime<Utc>,
) -> AgeCheckDecision {
    if declared_age >= age_minimum {
        return AgeCheckDecision::Grant;
    }
    let years = (age_minimum - declared_age).max(1);
    let days_per_year = i64::from(ban_days_per_year.max(1));
    let unban_at = now + Duration::days(i64::from(years) * days_per_year);
    let reason = format!("Verification d'age : {declared_age} ans (<{age_minimum}).");
    AgeCheckDecision::Ban {
        years,
        unban_at,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_when_age_meets_minimum() {
        let now = Utc::now();
        assert_eq!(decide_age_check(18, 18, 365, now), AgeCheckDecision::Grant);
        assert_eq!(decide_age_check(25, 18, 365, now), AgeCheckDecision::Grant);
    }

    #[test]
    fn ban_duration_matches_years_times_days_per_year() {
        let now = Utc::now();
        // 15 ans, minimum 18 -> 3 ans de ban -> 3 * 365 jours.
        match decide_age_check(15, 18, 365, now) {
            AgeCheckDecision::Ban {
                years,
                unban_at,
                reason,
            } => {
                assert_eq!(years, 3);
                assert_eq!(unban_at, now + Duration::days(3 * 365));
                assert_eq!(reason, "Verification d'age : 15 ans (<18).");
            }
            AgeCheckDecision::Grant => panic!("attendu Ban"),
        }
    }

    #[test]
    fn ban_years_floored_to_one() {
        let now = Utc::now();
        match decide_age_check(17, 18, 30, now) {
            AgeCheckDecision::Ban {
                years, unban_at, ..
            } => {
                assert_eq!(years, 1);
                assert_eq!(unban_at, now + Duration::days(30));
            }
            AgeCheckDecision::Grant => panic!("attendu Ban"),
        }
    }

    #[test]
    fn ban_days_per_year_floored_to_one() {
        let now = Utc::now();
        match decide_age_check(16, 18, 0, now) {
            AgeCheckDecision::Ban {
                years, unban_at, ..
            } => {
                assert_eq!(years, 2);
                assert_eq!(unban_at, now + Duration::days(2));
            }
            AgeCheckDecision::Grant => panic!("attendu Ban"),
        }
    }
}
