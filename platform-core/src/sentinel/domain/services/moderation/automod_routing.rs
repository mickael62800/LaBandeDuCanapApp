//! Decision de routage automod (full hexa) : DECIDE = API.
//!
//! Centralise la regle "que faire d'une detection ?" (carte de review / action
//! auto / rien), auparavant dupliquee cote bot. Le bot n'a plus qu'a EXECUTER
//! la decision retournee. Fonction pure : aucune I/O, testable directement.

use crate::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::sentinel::domain::enums::moderation::action::Action;

/// Que doit faire le bot de la detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Routing {
    /// Ne rien faire automatiquement (ex. human_only sans salon de review).
    None,
    /// Poster une carte de review/vote.
    Card,
    /// Appliquer directement l'action (mode auto, hors human_only).
    Auto,
}

/// Decision complete de routage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingDecision {
    pub route: Routing,
    /// Applique la sanction en plus de la carte de review. Cela permet de
    /// garder une trace et les boutons de moderation apres une action auto.
    pub auto_action: bool,
    /// Cas severe (phishing / invitation Discord) : protection auto immediate.
    pub severe: bool,
    /// Lien non autorise HORS image : suppression auto immediate.
    pub auto_delete_link: bool,
}

/// Reduit une sanction au niveau le plus severe explicitement autorise pour
/// l'automatisation. Une action plus faible n'est jamais augmentee : elle
/// reste donc soumise a la review si sa case n'est pas cochee.
pub fn cap_to_allowed_auto_action(
    action: &Action,
    selective: bool,
    auto_warn: bool,
    auto_delete: bool,
    auto_mute: bool,
    auto_kick: bool,
    auto_ban: bool,
) -> Action {
    if !selective || matches!(action, Action::None) {
        return action.clone();
    }

    [
        (Action::Warn, auto_warn),
        (Action::Delete, auto_delete),
        (Action::Mute, auto_mute),
        (Action::Kick, auto_kick),
        (Action::Ban, auto_ban),
    ]
    .into_iter()
    .filter(|(candidate, enabled)| *enabled && candidate <= action)
    .map(|(candidate, _)| candidate)
    .max()
    .unwrap_or_else(|| action.clone())
}

/// Entrees de la decision (faits + config guild deja resolue par l'API).
pub struct RoutingInputs<'a> {
    pub flags: &'a DetectionFlags,
    pub content: &'a str,
    pub score: f64,
    pub action: Action,
    pub human_only: bool,
    pub auto_protect: bool,
    pub auto_delete_links: bool,
    /// Autorisations explicites des sanctions executees sans carte.
    pub selective_auto_actions: bool,
    pub auto_warn: bool,
    pub auto_delete: bool,
    pub auto_mute: bool,
    pub auto_kick: bool,
    pub auto_ban: bool,
    pub ai_review_mode: bool,
    pub review_min_score: f64,
    /// `true` si un salon de review est configure (log_channel_id != 0).
    pub log_channel_set: bool,
}

/// `true` si l'invitation Discord (pub vers un autre serveur) est presente.
pub fn contains_discord_invite(content: &str) -> bool {
    let l = content.to_lowercase();
    l.contains("discord.gg/")
        || l.contains("discord.com/invite/")
        || l.contains("discordapp.com/invite/")
}

/// Cas "severe" justifiant une protection auto immediate meme en human_only :
/// phishing/scam ou invitation Discord.
pub fn is_severe_content(flags: &DetectionFlags, content: &str) -> bool {
    flags.phishing || contains_discord_invite(content)
}

const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "apng", "avif",
];

/// `true` si le message contient au moins une URL http(s) qui n'est PAS une
/// image (lien "hors image" a supprimer).
pub fn contains_non_image_url(content: &str) -> bool {
    content.split_whitespace().any(|tok| {
        let t = tok.trim_matches(|c: char| {
            !c.is_ascii_alphanumeric() && c != '/' && c != ':' && c != '.' && c != '-' && c != '_'
        });
        let lower = t.to_lowercase();
        if !(lower.starts_with("http://") || lower.starts_with("https://")) {
            return false;
        }
        let path = lower.split(['?', '#']).next().unwrap_or(&lower);
        let ext = path.rsplit('.').next().unwrap_or("");
        !IMAGE_EXTS.contains(&ext)
    })
}

/// Calcule la decision de routage a partir des faits + config guild.
///
/// Politique liens (CR revue moderation) : phishing / invitation Discord =
/// SEVERE (auto-protection). Un lien generique non autorise HORS image part
/// par defaut en CARTE (decision humaine) ; il n'est supprime automatiquement
/// que si `auto_delete_links` est explicitement active (mode agressif opt-in).
pub fn decide(i: &RoutingInputs) -> RoutingDecision {
    let severe = i.auto_protect && is_severe_content(i.flags, i.content);

    // Lien generique (hors phishing, hors image) detecte.
    let generic_link =
        !severe && i.flags.link && !i.flags.phishing && contains_non_image_url(i.content);
    // Suppression seche : uniquement si l'admin l'a explicitement demandee.
    let auto_delete_link = generic_link && i.auto_delete_links;
    // Sinon, le lien generique merite une carte (oeil humain).
    let link_needs_card = generic_link && !auto_delete_link;

    let above_threshold = i.score > 0.0 && i.score >= i.review_min_score;
    let action_is_authorized = match i.action {
        Action::None => false,
        // Warn/delete/mute preservent le mode auto historique tant que le
        // controle selectif n'a pas ete active.
        Action::Warn => !i.selective_auto_actions || i.auto_warn,
        Action::Delete => !i.selective_auto_actions || i.auto_delete,
        Action::Mute => !i.selective_auto_actions || i.auto_mute,
        Action::Kick => i.selective_auto_actions && i.auto_kick,
        // Le ban est toujours opt-in : il etait auparavant une proposition,
        // jamais un vrai bannissement automatique.
        Action::Ban => i.selective_auto_actions && i.auto_ban,
    };
    // `human_only` choisit le destinataire (une carte) mais ne transforme pas
    // chaque message normal en incident. Un score positif est indispensable :
    // sinon une IA indisponible ou sous son seuil produit des cartes à 0.00.
    let should_card = i.log_channel_set
        && ((i.human_only && i.score > 0.0)
            || severe
            || link_needs_card
            || (i.ai_review_mode && above_threshold)
            // Une sanction non autorisee n'est jamais appliquee en silence :
            // elle attend la decision des moderateurs dans la carte.
            || (!action_is_authorized && i.action != Action::None));

    let route = if auto_delete_link {
        // Le bot supprime via `auto_delete_link` ; pas d'autre action.
        Routing::None
    } else if should_card {
        Routing::Card
    } else if i.human_only {
        // Pas de carte (pas de salon) + human_only : aucune action auto.
        Routing::None
    } else if matches!(i.action, Action::None) || !action_is_authorized {
        Routing::None
    } else {
        Routing::Auto
    };

    RoutingDecision {
        route,
        auto_action: !auto_delete_link
            && !i.human_only
            && action_is_authorized
            && i.action != Action::None,
        severe,
        auto_delete_link,
    }
}

#[cfg(test)]
#[path = "tests/automod_routing.rs"]
mod tests;
