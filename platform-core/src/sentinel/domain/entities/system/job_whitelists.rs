//! Whitelists pour les jobs asynchrones (IA, exports). Centralise les
//! validations qui etaient eparpillees dans les handlers.

/// Types de job IA autorises. `analyze_text` pour la moderation de messages,
/// `analyze_image` pour la moderation d'images.
pub const VALID_AI_JOB_TYPES: &[&str] = &["analyze_text", "analyze_image"];

/// Verifie qu'un job_type d'IA est dans la whitelist.
pub fn is_valid_ai_job_type(job_type: &str) -> bool {
    VALID_AI_JOB_TYPES.contains(&job_type)
}

/// Types de job d'export autorises (table source).
pub const VALID_EXPORT_JOB_TYPES: &[&str] = &["infractions", "audit_logs", "moderation_actions"];

/// Verifie qu'un job_type d'export est dans la whitelist.
pub fn is_valid_export_job_type(job_type: &str) -> bool {
    VALID_EXPORT_JOB_TYPES.contains(&job_type)
}

/// Formats de sortie supportes pour un export.
pub const VALID_EXPORT_FORMATS: &[&str] = &["csv", "json"];

/// Verifie qu'un format d'export est dans la whitelist.
pub fn is_valid_export_format(format: &str) -> bool {
    VALID_EXPORT_FORMATS.contains(&format)
}

#[cfg(test)]
#[path = "tests/job_whitelists.rs"]
mod tests;
