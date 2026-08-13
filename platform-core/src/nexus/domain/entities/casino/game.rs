//! Regles metier pures liees aux jeux (games) — validation nom, normalisation
//! des tags optionnels (emoji/category), parsing couleur de role.

/// Longueur max d'un nom de jeu.
pub const MAX_GAME_NAME_LEN: usize = 100;

/// Couleur par defaut pour les roles Discord crees automatiquement pour les jeux.
/// Valeur hex 0x3498DB (bleu "peter river").
pub const DEFAULT_GAME_ROLE_COLOR: u32 = 0x3498DB;

/// Normalise un nom de jeu : trim + validation non-vide + longueur max.
///
/// Retourne `Ok(trimmed_name)` si valide, sinon un message d'erreur statique
/// que le handler enrobera dans un `DomainError::ValidationError`.
pub fn normalize_game_name(raw: &str) -> Result<String, &'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Le nom du jeu ne peut pas etre vide");
    }
    if trimmed.chars().count() > MAX_GAME_NAME_LEN {
        return Err("Le nom du jeu ne peut pas depasser 100 caracteres");
    }
    Ok(trimmed.to_string())
}

/// Normalise un tag optionnel (emoji, category) : trim + filter empty.
/// Si apres trim le tag est vide, retourne `None`.
pub fn normalize_optional_tag(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Parse une couleur hex Discord (`#RRGGBB` ou `RRGGBB`) avec fallback en
/// cas d'input invalide. Accepte les espaces autour.
pub fn parse_role_color_hex(hex: &str, fallback: u32) -> u32 {
    u32::from_str_radix(hex.trim().trim_start_matches('#'), 16).unwrap_or(fallback)
}

// ── Regles Discord pour les emojis custom ─────────────────────

/// Taille max d'une image d'emoji Discord (256 KB).
pub const MAX_EMOJI_IMAGE_BYTES: usize = 256 * 1024;

/// MIME types autorises par Discord pour les emojis custom.
pub const ALLOWED_EMOJI_MIMES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/jpg",
    "image/gif",
    "image/webp",
];

/// Verifie si un MIME type est autorise par Discord pour un emoji custom.
pub fn is_allowed_emoji_mime(mime: &str) -> bool {
    ALLOWED_EMOJI_MIMES.contains(&mime)
}

/// Convertit un nom arbitraire en slug valide Discord pour un emoji :
/// - ne conserve que [A-Za-z0-9_]
/// - remplace whitespace/`-`/`.` par `_` (collapsing des sequences)
/// - trim les `_` en debut/fin
/// - truncate a 32 chars (limite Discord)
/// - pad a 2 chars si trop court (limite Discord)
///
/// Regle metier : contrainte de nom Discord (2..=32 chars, alphanum + `_`).
pub fn slugify_emoji_name(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '_'
            || ((ch.is_whitespace() || ch == '-' || ch == '.') && !out.ends_with('_'))
        {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    let mut s = trimmed;
    if s.len() > 32 {
        s.truncate(32);
    }
    while s.len() < 2 {
        s.push('_');
    }
    s
}

/// Formate un emoji custom Discord : `<a:name:id>` si anime, `<:name:id>` sinon.
/// Regle metier : syntaxe d'insertion Discord pour un emoji custom.
pub fn format_custom_emoji(name: &str, id: &str, animated: bool) -> String {
    if animated {
        format!("<a:{}:{}>", name, id)
    } else {
        format!("<:{}:{}>", name, id)
    }
}

#[cfg(test)]
#[path = "tests/game.rs"]
mod tests;
