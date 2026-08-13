//! Regles metier pour les evidences et review queue de moderation.

/// Longueur max d'une URL de preuve (evidence). Choix pragmatique (Discord
/// n'a pas de limite stricte ; on s'aligne sur un URL "raisonnable").
pub const MAX_EVIDENCE_URL_LEN: usize = 2000;

/// Longueur max d'un texte de description/reason/notes de review.
/// Truncate silencieusement si depassee.
pub const MAX_REVIEW_TEXT_LEN: usize = 500;

/// Duree par defaut d'un mute timeout en secondes (1 heure).
pub const DEFAULT_MUTE_DURATION_SECS: u64 = 3600;

/// Resout la duree de mute a appliquer : defaut 1h si non fournie.
/// Centralise la regle "absent -> DEFAULT_MUTE_DURATION_SECS" pour eviter
/// que chaque call site (handler HTTP, bot, worker) la redefinisse.
pub fn resolve_mute_duration(input: Option<u64>) -> u64 {
    input.unwrap_or(DEFAULT_MUTE_DURATION_SECS)
}

/// Valide une URL de preuve : trim non-vide et longueur <= 2000.
pub fn validate_evidence_url(url: &str) -> Result<(), &'static str> {
    if url.trim().is_empty() || url.len() > MAX_EVIDENCE_URL_LEN {
        return Err("url vide ou trop longue (max 2000)");
    }
    Ok(())
}

/// Tronque un texte de review (description, reason, notes) a 500 caracteres.
/// Utilise `chars().take(...)` pour compter les graphemes (Unicode-safe).
pub fn truncate_review_text(s: &str) -> String {
    s.chars().take(MAX_REVIEW_TEXT_LEN).collect()
}

/// Statuts valides pour la resolution d'une review.
pub const VALID_REVIEW_STATUSES: &[&str] = &["approved", "rejected", "changed"];

/// Valide qu'un statut de review est dans la whitelist.
pub fn is_valid_review_status(status: &str) -> bool {
    VALID_REVIEW_STATUSES.contains(&status)
}

#[cfg(test)]
#[path = "tests/manual.rs"]
mod tests;
