//! Règles d'alerte de supervision (table `alert_rules`), pilotant le
//! dispatcher d'alertes host-level.

use crate::domain::errors::DomainError;

/// Cooldown minimal entre deux déclenchements d'une même règle (anti-spam).
pub const ALERT_RULE_MIN_COOLDOWN_SECS: i32 = 60;

/// Sévérités autorisées pour une règle d'alerte.
pub const ALERT_SEVERITIES: &[&str] = &["info", "warning", "critical"];

#[derive(Debug, Clone)]
pub struct AlertRule {
    pub id: String,
    pub label: String,
    pub metric: String,
    pub comparator: String,
    pub threshold: Option<f64>,
    pub enabled: bool,
    pub severity: String,
    pub cooldown_secs: i32,
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
