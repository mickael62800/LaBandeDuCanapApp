//! Evaluation du RISQUE d'une cible avant une action de moderation destructive.
//!
//! Garde-fou UX : decide si une action (ban / mute) exige une confirmation
//! explicite du moderateur. La regle metier PURE vit ici (plus dans le bot) :
//! a partir des FAITS Discord collectes par le bot (age du compte, cible=bot,
//! cible=membre du staff) et du SEUIL serveur (`recent_account_days`), on decide
//! `risky: oui/non + raison`. Le bot ne fournit que les faits ; la politique
//! (quels faits declenchent la confirmation) et le seuil sont server-side.

/// Faits Discord collectes par le bot sur la cible. Aucune donnee Discord n'est
/// requise cote core : le bot les resout (age via `created_at`, `is_bot` via le
/// user, `has_mod_perms` via un scan des permissions des roles).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetRiskFacts {
    /// Age du compte Discord en jours (deja borne a >= 0 par l'appelant).
    pub account_age_days: i64,
    /// La cible est un bot.
    pub is_bot: bool,
    /// La cible possede des permissions de moderation (staff).
    pub has_mod_perms: bool,
}

/// Decision de risque : `risky` arme la modale de confirmation, `reason` en
/// donne le motif (affiche au moderateur). `reason` est `None` ssi `!risky`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetRiskDecision {
    pub risky: bool,
    pub reason: Option<String>,
}

impl TargetRiskDecision {
    fn risky(reason: impl Into<String>) -> Self {
        Self {
            risky: true,
            reason: Some(reason.into()),
        }
    }

    fn safe() -> Self {
        Self {
            risky: false,
            reason: None,
        }
    }
}

/// Regle metier PURE de l'evaluation de risque de cible.
///
/// Priorite des motifs (identique au comportement bot historique) :
/// 1. compte Discord recent (age < `recent_account_days`),
/// 2. cible = bot,
/// 3. cible = membre de l'equipe de moderation.
/// Sinon : non risque.
pub fn decide_target_risk(facts: &TargetRiskFacts, recent_account_days: i64) -> TargetRiskDecision {
    let age = facts.account_age_days.max(0);
    if age < recent_account_days {
        return TargetRiskDecision::risky(format!(
            "compte Discord cree il y a seulement {age} jour(s)"
        ));
    }
    if facts.is_bot {
        return TargetRiskDecision::risky("cible est un bot");
    }
    if facts.has_mod_perms {
        return TargetRiskDecision::risky("cible fait partie de l'equipe de moderation");
    }
    TargetRiskDecision::safe()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_DAYS: i64 = 7;

    fn facts(account_age_days: i64, is_bot: bool, has_mod_perms: bool) -> TargetRiskFacts {
        TargetRiskFacts {
            account_age_days,
            is_bot,
            has_mod_perms,
        }
    }

    #[test]
    fn recent_account_is_risky() {
        let d = decide_target_risk(&facts(3, false, false), DEFAULT_DAYS);
        assert!(d.risky);
        assert_eq!(
            d.reason.as_deref(),
            Some("compte Discord cree il y a seulement 3 jour(s)")
        );
    }

    #[test]
    fn account_exactly_at_threshold_is_safe() {
        // age == seuil : non risque (strictement inferieur seulement).
        let d = decide_target_risk(&facts(DEFAULT_DAYS, false, false), DEFAULT_DAYS);
        assert!(!d.risky);
        assert!(d.reason.is_none());
    }

    #[test]
    fn negative_age_clamped_to_zero() {
        let d = decide_target_risk(&facts(-5, false, false), DEFAULT_DAYS);
        assert!(d.risky);
        assert_eq!(
            d.reason.as_deref(),
            Some("compte Discord cree il y a seulement 0 jour(s)")
        );
    }

    #[test]
    fn bot_target_is_risky() {
        let d = decide_target_risk(&facts(365, true, false), DEFAULT_DAYS);
        assert!(d.risky);
        assert_eq!(d.reason.as_deref(), Some("cible est un bot"));
    }

    #[test]
    fn mod_member_is_risky() {
        let d = decide_target_risk(&facts(365, false, true), DEFAULT_DAYS);
        assert!(d.risky);
        assert_eq!(
            d.reason.as_deref(),
            Some("cible fait partie de l'equipe de moderation")
        );
    }

    #[test]
    fn normal_target_is_safe() {
        let d = decide_target_risk(&facts(365, false, false), DEFAULT_DAYS);
        assert!(!d.risky);
        assert!(d.reason.is_none());
    }

    #[test]
    fn recent_account_takes_priority_over_bot_and_mod() {
        let d = decide_target_risk(&facts(1, true, true), DEFAULT_DAYS);
        assert_eq!(
            d.reason.as_deref(),
            Some("compte Discord cree il y a seulement 1 jour(s)")
        );
    }

    #[test]
    fn configurable_threshold_is_honored() {
        // Seuil serveur eleve (30j) : un compte de 10j devient risque.
        let d = decide_target_risk(&facts(10, false, false), 30);
        assert!(d.risky);
    }
}
