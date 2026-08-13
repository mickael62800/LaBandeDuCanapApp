//! Tests de la jurisprudence du copilote : action modale et departage.

use super::*;

fn dist(pairs: &[(&str, u32)]) -> PrecedentDistribution {
    let counts: Vec<(String, u32)> = pairs.iter().map(|(a, c)| (a.to_string(), *c)).collect();
    let total = counts.iter().map(|(_, c)| *c).sum();
    PrecedentDistribution {
        flag_category: "spam".to_string(),
        counts_by_action: counts,
        total,
    }
}

#[test]
fn empty_has_no_modal_action() {
    let d = PrecedentDistribution::empty("insult");
    assert_eq!(d.flag_category, "insult");
    assert_eq!(d.total, 0);
    assert_eq!(d.modal_action(), None);
}

#[test]
fn modal_picks_most_frequent() {
    let d = dist(&[("warn", 5), ("ban", 2), ("mute", 1)]);
    assert_eq!(d.modal_action(), Some(AppliedAction::Warn));
}

#[test]
fn modal_tie_breaks_toward_most_severe() {
    // warn et ban a egalite (3 chacun) -> departage vers le plus severe (ban).
    let d = dist(&[("warn", 3), ("ban", 3)]);
    assert_eq!(d.modal_action(), Some(AppliedAction::Ban));
}

#[test]
fn modal_tie_break_mute_over_delete() {
    let d = dist(&[("delete", 2), ("mute", 2)]);
    assert_eq!(d.modal_action(), Some(AppliedAction::Mute));
}

#[test]
fn modal_ignores_unknown_actions() {
    // "foobar" n'est pas une action reconnue -> ignoree ; warn l'emporte.
    let d = dist(&[("foobar", 99), ("warn", 1)]);
    assert_eq!(d.modal_action(), Some(AppliedAction::Warn));
}

#[test]
fn modal_all_unknown_is_none() {
    let d = dist(&[("nope", 3), ("???", 1)]);
    assert_eq!(d.modal_action(), None);
}

#[test]
fn suggestion_basis_as_str_roundtrip() {
    assert_eq!(SuggestionBasis::Escalation.as_str(), "escalation");
    assert_eq!(SuggestionBasis::Precedent.as_str(), "precedent");
    assert_eq!(SuggestionBasis::Both.as_str(), "both");
    assert_eq!(SuggestionBasis::Insufficient.as_str(), "insufficient");
}
