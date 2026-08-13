//! Tests unitaires du service de domaine pur `suggest_sanction`.
//! Couvre toutes les branches : precedents suffisants/insuffisants, historique
//! vide, bornes d'escalade, egalite de precedents.

use super::*;
use crate::sentinel::domain::entities::moderation::action::strikes::StrikeThreshold;
use crate::sentinel::domain::entities::moderation::copilot::PrecedentDistribution;
use crate::sentinel::domain::entities::moderation::copilot::SuggestionBasis;
use crate::sentinel::domain::entities::moderation::review::automod::AppliedAction;

fn threshold(strikes: u32, action: &str) -> StrikeThreshold {
    StrikeThreshold {
        strikes,
        action: action.to_string(),
        duration: None,
    }
}

fn precedents(category: &str, counts: &[(&str, u32)]) -> PrecedentDistribution {
    let counts_by_action: Vec<(String, u32)> =
        counts.iter().map(|(a, c)| (a.to_string(), *c)).collect();
    let total = counts.iter().map(|(_, c)| *c).sum();
    PrecedentDistribution {
        flag_category: category.to_string(),
        counts_by_action,
        total,
    }
}

// ── Precedents suffisants : on suit la jurisprudence ──────────────────

#[test]
fn precedents_suffisants_avec_escalade_donne_both() {
    let thresholds = vec![threshold(1, "warn"), threshold(3, "ban")];
    let prec = precedents("spam", &[("mute", 4), ("warn", 1)]);
    let out = suggest_sanction(&SuggestInputs {
        active_strikes: 2, // next=3 -> ban
        thresholds: &thresholds,
        precedents: &prec,
        min_precedents: 3,
    });
    assert_eq!(out.basis, SuggestionBasis::Both);
    assert_eq!(out.action, Some(AppliedAction::Mute)); // modale, pas l'escalade
    assert_eq!(out.precedent_count, 5);
    assert!(out.rationale.contains("Jurisprudence"));
    assert!(out.rationale.contains("ban")); // cite l'escalade
}

#[test]
fn precedents_suffisants_sans_escalade_donne_precedent() {
    let thresholds: Vec<StrikeThreshold> = vec![]; // pas de ladder
    let prec = precedents("insult", &[("warn", 3)]);
    let out = suggest_sanction(&SuggestInputs {
        active_strikes: 0,
        thresholds: &thresholds,
        precedents: &prec,
        min_precedents: 2,
    });
    assert_eq!(out.basis, SuggestionBasis::Precedent);
    assert_eq!(out.action, Some(AppliedAction::Warn));
    assert!(out.rationale.contains("Aucune escalade"));
}

#[test]
fn egalite_precedents_departage_vers_la_plus_severe() {
    let thresholds: Vec<StrikeThreshold> = vec![];
    // warn (severity 2) vs ban (severity 5), meme compte -> ban gagne.
    let prec = precedents("spam", &[("warn", 2), ("ban", 2)]);
    let out = suggest_sanction(&SuggestInputs {
        active_strikes: 0,
        thresholds: &thresholds,
        precedents: &prec,
        min_precedents: 1,
    });
    assert_eq!(out.action, Some(AppliedAction::Ban));
    assert_eq!(out.basis, SuggestionBasis::Precedent);
}

// ── Precedents insuffisants : on retombe sur l'escalade ───────────────

#[test]
fn precedents_insuffisants_utilise_escalade() {
    let thresholds = vec![
        threshold(1, "warn"),
        threshold(2, "mute"),
        threshold(4, "ban"),
    ];
    let prec = precedents("spam", &[("ban", 1)]); // total=1 < min=3
    let out = suggest_sanction(&SuggestInputs {
        active_strikes: 1, // next=2 -> mute
        thresholds: &thresholds,
        precedents: &prec,
        min_precedents: 3,
    });
    assert_eq!(out.basis, SuggestionBasis::Escalation);
    assert_eq!(out.action, Some(AppliedAction::Mute));
    assert!(out.rationale.contains("pas assez de precedents"));
    assert_eq!(out.precedent_count, 1);
}

#[test]
fn historique_vide_et_escalade_disponible() {
    let thresholds = vec![threshold(1, "warn")];
    let prec = PrecedentDistribution::empty("");
    let out = suggest_sanction(&SuggestInputs {
        active_strikes: 0, // next=1 -> warn
        thresholds: &thresholds,
        precedents: &prec,
        min_precedents: 2,
    });
    assert_eq!(out.basis, SuggestionBasis::Escalation);
    assert_eq!(out.action, Some(AppliedAction::Warn));
    assert!(out.rationale.contains("aucun precedent disponible"));
}

// ── Aucune base : Insufficient ────────────────────────────────────────

#[test]
fn ni_escalade_ni_precedents_donne_insufficient() {
    let thresholds: Vec<StrikeThreshold> = vec![]; // pas de ladder
    let prec = PrecedentDistribution::empty("spam");
    let out = suggest_sanction(&SuggestInputs {
        active_strikes: 0,
        thresholds: &thresholds,
        precedents: &prec,
        min_precedents: 1,
    });
    assert_eq!(out.basis, SuggestionBasis::Insufficient);
    assert_eq!(out.action, None);
    assert!(out.rationale.contains("Aucune suggestion"));
}

// ── Bornes d'escalade ─────────────────────────────────────────────────

#[test]
fn borne_escalade_juste_en_dessous_du_seuil() {
    // Seuil ban a 3. active=1 -> next=2 < 3 -> pas de ban, mute (seuil 2) atteint.
    let thresholds = vec![threshold(2, "mute"), threshold(3, "ban")];
    let prec = PrecedentDistribution::empty("");
    let out = suggest_sanction(&SuggestInputs {
        active_strikes: 1,
        thresholds: &thresholds,
        precedents: &prec,
        min_precedents: 1,
    });
    assert_eq!(out.action, Some(AppliedAction::Mute));
}

#[test]
fn borne_escalade_sous_le_premier_seuil_pas_d_action() {
    // Premier seuil a 2. active=0 -> next=1 < 2 -> aucune escalade.
    let thresholds = vec![threshold(2, "mute")];
    let prec = PrecedentDistribution::empty("");
    let out = suggest_sanction(&SuggestInputs {
        active_strikes: 0,
        thresholds: &thresholds,
        precedents: &prec,
        min_precedents: 1,
    });
    assert_eq!(out.basis, SuggestionBasis::Insufficient);
    assert_eq!(out.action, None);
}

#[test]
fn min_precedents_zero_est_clamp_a_un() {
    let thresholds: Vec<StrikeThreshold> = vec![];
    let prec = precedents("spam", &[("warn", 1)]); // total=1
    let out = suggest_sanction(&SuggestInputs {
        active_strikes: 0,
        thresholds: &thresholds,
        precedents: &prec,
        min_precedents: 0, // clamp -> 1, donc 1 >= 1 suffisant
    });
    assert_eq!(out.basis, SuggestionBasis::Precedent);
    assert_eq!(out.action, Some(AppliedAction::Warn));
}
