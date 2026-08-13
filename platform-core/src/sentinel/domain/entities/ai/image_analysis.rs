use crate::sentinel::domain::enums::moderation::action::Action;

/// Taille max (en octets bruts base64) d'une image acceptee par l'API IA.
/// ~10 Mo d'image binaire ~= 13.3 Mo en base64.
pub const MAX_IMAGE_BASE64_LEN: usize = 14_000_000;

/// Content-types image autorises par l'API IA.
pub const ALLOWED_IMAGE_CONTENT_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/bmp",
];

/// Verifie si un content-type est dans la liste autorisee (case-insensitive
/// sur le content-type d'entree, les constantes restent en minuscules).
pub fn is_allowed_image_content_type(content_type: &str) -> bool {
    let lowered = content_type.to_ascii_lowercase();
    ALLOWED_IMAGE_CONTENT_TYPES.contains(&lowered.as_str())
}

/// Verifie si la taille base64 d'une image est dans les limites acceptables.
pub fn is_image_size_acceptable(size_bytes: usize) -> bool {
    size_bytes <= MAX_IMAGE_BASE64_LEN
}

#[cfg(test)]
#[path = "tests/image_analysis.rs"]
mod tests;

/// Resultat de l'analyse d'une image par le modele vision ONNX.
#[derive(Debug, Clone)]
pub struct ImageAnalysis {
    pub action: Action,
    pub reason: String,
    pub score: f64,
    pub duration: Option<u64>,
    pub classifications: Vec<ImageClassification>,
}

#[derive(Debug, Clone)]
pub struct ImageClassification {
    pub label: String,
    pub confidence: f32,
}
