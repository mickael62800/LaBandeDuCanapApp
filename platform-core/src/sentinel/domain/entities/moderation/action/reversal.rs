//! Infos minimales necessaires pour annuler (reverser) une action de
//! moderation : derivees d'une ligne `audit_logs` (event_type `mod_*`).

/// `action_type` est deja stripe du prefixe `mod_` (ex: `ban_permanent`).
#[derive(Debug, Clone)]
pub struct ActionReversalInfo {
    pub guild_id: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
}

/// Effet Discord inverse a appliquer lors de l'annulation d'une action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReversalEffect {
    /// `ban*` : unban + annulation du rappel d'auto-unban encore pending
    /// (sinon le worker `expire_temp_bans` rejouerait un unban tardif).
    Unban { cancel_auto_unban_reminder: bool },
    /// `mute*` / `timeout` : retirer le timeout Discord.
    RemoveTimeout,
    /// `warn` / autre : aucun effet Discord natif, suppression DB seule.
    None,
}

/// Regle de reversibilite : quel effet Discord inverse pour ce type d'action.
pub fn reversal_effect(action_type: &str) -> ReversalEffect {
    let lower = action_type.to_lowercase();
    if lower.starts_with("ban") {
        ReversalEffect::Unban {
            cancel_auto_unban_reminder: true,
        }
    } else if lower.starts_with("mute") || lower == "timeout" {
        ReversalEffect::RemoveTimeout
    } else {
        ReversalEffect::None
    }
}

/// Fenetre effective (secondes) du quota d'actions par moderateur
/// (garde-fou anti-modo compromis) : defaut 1h, bornee 1s..24h.
pub fn mod_action_window_secs(requested: Option<i64>) -> i64 {
    requested.unwrap_or(3600).clamp(1, 86_400)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ban_variants_reverse_to_unban() {
        for t in ["ban_permanent", "ban_temp", "BAN_7D"] {
            assert_eq!(
                reversal_effect(t),
                ReversalEffect::Unban {
                    cancel_auto_unban_reminder: true
                }
            );
        }
    }

    #[test]
    fn mute_and_timeout_reverse_to_remove_timeout() {
        for t in ["mute", "mute_1h", "timeout", "Timeout"] {
            assert_eq!(reversal_effect(t), ReversalEffect::RemoveTimeout);
        }
    }

    #[test]
    fn warn_and_unknown_have_no_effect() {
        for t in ["warn", "kick", "note", ""] {
            assert_eq!(reversal_effect(t), ReversalEffect::None);
        }
    }

    #[test]
    fn window_default_and_bounds() {
        assert_eq!(mod_action_window_secs(None), 3600);
        assert_eq!(mod_action_window_secs(Some(0)), 1);
        assert_eq!(mod_action_window_secs(Some(100_000)), 86_400);
        assert_eq!(mod_action_window_secs(Some(600)), 600);
    }
}
