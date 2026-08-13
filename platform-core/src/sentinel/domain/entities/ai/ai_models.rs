//! Regles metier autour des modeles ONNX charges par le service d'inference :
//! - types de modeles supportes (`vision`, `text`) — whitelist partagee par
//!   GET /api/models/status et POST /api/models/reload.
//! - formatage du nom d'affichage (basename du chemin + fallback
//!   "non configure"), utilise par GET /api/models/status.

/// Types de modeles ONNX supportes par le service d'inference.
pub const SUPPORTED_MODEL_TYPES: &[&str] = &["vision", "text"];

pub fn is_valid_model_type(s: &str) -> bool {
    SUPPORTED_MODEL_TYPES.contains(&s)
}

/// Extrait le basename d'un chemin (dernier segment apres `/` ou `\`).
/// Retourne le chemin complet si aucun separateur. Accepte les chemins
/// Windows et Unix.
pub fn path_basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Construit le nom d'affichage d'un modele pour l'UI admin :
/// - `"{kind_label} ONNX (non configure)"` si le chemin est vide
/// - `"{kind_label} ONNX ({basename})"` sinon
pub fn format_model_display_name(kind_label: &str, path: &str) -> String {
    if path.is_empty() {
        format!("{kind_label} ONNX (non configure)")
    } else {
        format!("{kind_label} ONNX ({})", path_basename(path))
    }
}

#[cfg(test)]
#[path = "tests/ai_models.rs"]
mod tests;
