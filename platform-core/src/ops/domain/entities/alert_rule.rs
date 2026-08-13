//! Règles d'alerte de supervision (table `alert_rules`), pilotant le
//! dispatcher d'alertes host-level.

use crate::ops::domain::errors::DomainError;

/// Cooldown minimal entre deux déclenchements d'une même règle (anti-spam).
pub const ALERT_RULE_MIN_COOLDOWN_SECS: i32 = 60;

/// Sévérités autorisées pour une règle d'alerte.
pub const ALERT_SEVERITIES: &[&str] = &["info", "warning", "critical"];

#[derive(Debug, Clone)]
pub struct AlertRule {
    /// Identifiant stable de la règle.
    pub id: String,
    /// Libellé affiché aux administrateurs.
    pub label: String,
    /// Métrique évaluée par le worker.
    pub metric: String,
    /// Comparateur supporté : `gt` ou `lt`.
    pub comparator: String,
    /// Seuil numérique ; absent pour les métriques booléennes.
    pub threshold: Option<f64>,
    /// Règle prise en compte par le dispatcher.
    pub enabled: bool,
    /// Niveau de notification : info, warning ou critical.
    pub severity: String,
    /// Délai minimal entre deux notifications.
    pub cooldown_secs: i32,
}

impl AlertRule {
    /// `true` si la valeur observée franchit le seuil de la règle.
    ///
    /// Cette décision vivait dans le dispatcher d'alertes, sur une copie
    /// `sqlx::FromRow` de cette entité : deux définitions d'`AlertRule`
    /// coexistaient, et seule celle du dispatcher savait décider. C'est la
    /// règle métier centrale du module, elle appartient au domaine.
    ///
    /// Un comparateur inconnu ou un seuil absent ne déclenchent jamais : une
    /// règle qu'on ne sait pas évaluer doit rester silencieuse, pas alerter à
    /// tort. Les métriques booléennes gardent d'ailleurs `threshold` à NULL.
    pub fn triggers(&self, value: f64) -> bool {
        match (self.comparator.as_str(), self.threshold) {
            ("gt", Some(seuil)) => value > seuil,
            ("lt", Some(seuil)) => value < seuil,
            _ => false,
        }
    }

    /// Couleur d'embed Discord associée à la sévérité.
    pub fn color(&self) -> u32 {
        match self.severity.as_str() {
            "critical" => 0xED4245,
            "warning" => 0xFEE75C,
            _ => 0x5865F2,
        }
    }
}

/// Champs éditables d'une règle. `metric`/`comparator`/`label` sont fixes
/// (ils définissent la sémantique de la règle).
#[derive(Debug, Clone, Default)]
pub struct AlertRuleUpdate {
    pub enabled: Option<bool>,
    pub threshold: Option<f64>,
    pub severity: Option<String>,
    pub cooldown_secs: Option<i32>,
}

impl AlertRuleUpdate {
    /// Invariants métier : sévérité dans la liste autorisée, cooldown ≥ 60s.
    pub fn validate(&self) -> Result<(), DomainError> {
        if let Some(ref s) = self.severity {
            if !ALERT_SEVERITIES.contains(&s.as_str()) {
                return Err(DomainError::ValidationError(
                    "severite invalide (info|warning|critical)".into(),
                ));
            }
        }
        if let Some(c) = self.cooldown_secs {
            if c < ALERT_RULE_MIN_COOLDOWN_SECS {
                return Err(DomainError::ValidationError(
                    "cooldown_secs minimum 60".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_update_valid() {
        assert!(AlertRuleUpdate::default().validate().is_ok());
    }

    #[test]
    fn valid_severities_accepted() {
        for s in ["info", "warning", "critical"] {
            let u = AlertRuleUpdate {
                severity: Some(s.into()),
                ..Default::default()
            };
            assert!(u.validate().is_ok(), "{s} devrait être valide");
        }
    }

    #[test]
    fn unknown_severity_rejected() {
        let u = AlertRuleUpdate {
            severity: Some("fatal".into()),
            ..Default::default()
        };
        assert!(u.validate().is_err());
    }

    fn regle(comparator: &str, threshold: Option<f64>) -> AlertRule {
        AlertRule {
            id: "r".into(),
            label: "Regle".into(),
            metric: "cpu".into(),
            comparator: comparator.into(),
            threshold,
            enabled: true,
            severity: "warning".into(),
            cooldown_secs: 300,
        }
    }

    #[test]
    fn seuil_franchi_vers_le_haut() {
        let r = regle("gt", Some(80.0));
        assert!(r.triggers(80.1));
        assert!(!r.triggers(80.0), "l'egalite ne franchit pas le seuil");
        assert!(!r.triggers(79.9));
    }

    #[test]
    fn seuil_franchi_vers_le_bas() {
        let r = regle("lt", Some(10.0));
        assert!(r.triggers(9.9));
        assert!(!r.triggers(10.0));
    }

    #[test]
    fn regle_inevaluable_reste_silencieuse() {
        // Seuil absent (metrique booleenne) ou comparateur inconnu : ne jamais
        // alerter a tort vaut mieux que reveiller quelqu'un pour rien.
        assert!(!regle("gt", None).triggers(999.0));
        assert!(!regle("entre", Some(1.0)).triggers(999.0));
    }

    #[test]
    fn couleur_suit_la_severite() {
        let mut r = regle("gt", Some(1.0));
        r.severity = "critical".into();
        assert_eq!(r.color(), 0xED4245);
        r.severity = "info".into();
        assert_eq!(r.color(), 0x5865F2);
    }

    #[test]
    fn cooldown_floor_enforced() {
        let below = AlertRuleUpdate {
            cooldown_secs: Some(59),
            ..Default::default()
        };
        assert!(below.validate().is_err());
        let at = AlertRuleUpdate {
            cooldown_secs: Some(60),
            ..Default::default()
        };
        assert!(at.validate().is_ok());
    }
}
