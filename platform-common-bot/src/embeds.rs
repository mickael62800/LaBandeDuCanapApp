//! Helpers d'embeds Discord uniformes pour tous les bots Sentinel.
//!
//! Garantit une coherence visuelle (couleurs, footer, timestamp) sur tous les bots.

use serenity::all::{CreateEmbed, CreateEmbedFooter, Timestamp};

// ── Palette de couleurs ──

/// Blurple Discord — info, stats, niveaux, roles
pub const COLOR_INFO: u32 = 0x5865F2;
/// Vert — actions reussies, bienvenue, verification OK
pub const COLOR_SUCCESS: u32 = 0x57F287;
/// Jaune — avertissements, warns, rappels
pub const COLOR_WARNING: u32 = 0xFEE75C;
/// Orange — mute, suppression, sanctions legeres
pub const COLOR_MODERATE: u32 = 0xF97316;
/// Rouge — ban, raid, contenu illicite
pub const COLOR_DANGER: u32 = 0xED4245;
/// Rouge sombre — ban permanent, alerte critique
pub const COLOR_CRITICAL: u32 = 0xDC2626;
/// Gris — messages systeme, infos neutres
pub const COLOR_NEUTRAL: u32 = 0x95A5A6;

// ── Builders ──

/// Cree un embed Sentinel avec footer et timestamp automatiques.
pub fn sentinel_embed(title: impl Into<String>, color: u32) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .color(color)
        .footer(CreateEmbedFooter::new("Sentinel"))
        .timestamp(Timestamp::now())
}

// ── Variantes pre-configurees ──

/// Embed info (blurple) — stats, niveaux, roles
pub fn info_embed(title: impl Into<String>) -> CreateEmbed {
    sentinel_embed(title, COLOR_INFO)
}

/// Embed succes (vert) — action reussie, bienvenue
pub fn success_embed(title: impl Into<String>) -> CreateEmbed {
    sentinel_embed(title, COLOR_SUCCESS)
}

/// Embed avertissement (jaune) — warn, rappel
pub fn warn_embed(title: impl Into<String>) -> CreateEmbed {
    sentinel_embed(title, COLOR_WARNING)
}

/// Embed moderation (orange) — mute, suppression
pub fn moderate_embed(title: impl Into<String>) -> CreateEmbed {
    sentinel_embed(title, COLOR_MODERATE)
}

/// Embed danger (rouge) — ban, raid, contenu interdit
pub fn danger_embed(title: impl Into<String>) -> CreateEmbed {
    sentinel_embed(title, COLOR_DANGER)
}

/// Embed critique (rouge sombre) — ban permanent, alerte critique
pub fn critical_embed(title: impl Into<String>) -> CreateEmbed {
    sentinel_embed(title, COLOR_CRITICAL)
}

/// Embed neutre (gris) — message systeme, info neutre
pub fn neutral_embed(title: impl Into<String>) -> CreateEmbed {
    sentinel_embed(title, COLOR_NEUTRAL)
}

/// Embed de feedback ephemere (retour d'une commande/interaction a son auteur).
/// La couleur est deduite de l'emoji en tete de message, qui encode deja la
/// nature du retour :
///   ✅ succes -> vert | ⚠️/⏳ transitoire -> orange | ❌/⛔ erreur -> rouge |
///   sinon -> gris neutre.
/// Le message va dans la description (les mentions n'y declenchent pas de ping,
/// ce qui convient a un retour ephemere).
pub fn feedback_embed(message: impl Into<String>) -> CreateEmbed {
    let message = message.into();
    let head = message.trim_start();
    let color = if head.starts_with('✅') {
        COLOR_SUCCESS
    } else if head.starts_with('⚠') || head.starts_with('⏳') {
        COLOR_MODERATE
    } else if head.starts_with('❌') || head.starts_with('⛔') {
        COLOR_DANGER
    } else {
        COLOR_NEUTRAL
    };
    CreateEmbed::new()
        .description(message)
        .color(color)
        .footer(CreateEmbedFooter::new("Sentinel"))
        .timestamp(Timestamp::now())
}

// ── Gabarit de message de sanction (destiné au membre) ──

/// Gabarit UNIFORME d'un message de sanction adressé au membre.
///
/// Ton cohérent quel que soit le chemin (automod auto, review 1-clic, vote) et
/// **mention systématique du droit d'appel** (`/appeal`) quand `appeal` est vrai
/// — conformité DSA. `action` ∈ prevention|warn|delete|mute|ban.
pub fn sanction_notice(
    action: &str,
    reason: &str,
    mute_minutes: Option<u64>,
    validated_by: Option<&str>,
    appeal: bool,
) -> CreateEmbed {
    let title = match action {
        "prevention" => "Mesure de prévention",
        "warn" => "Avertissement",
        "delete" => "Message supprimé",
        "mute" => "Exclusion temporaire (mute)",
        "kick" => "Exclusion du serveur (kick)",
        "ban" => "Bannissement",
        _ => "Décision de modération",
    };
    let mut e = sentinel_embed(
        format!("{} {}", action_emoji(action), title),
        action_color(action),
    )
    .field(
        "Raison",
        if reason.trim().is_empty() {
            "Non précisée"
        } else {
            reason
        },
        false,
    );
    if let Some(m) = mute_minutes {
        e = e.field("Durée", format!("{m} minute(s)"), true);
    }
    if let Some(by) = validated_by {
        e = e.field("Validé par", by, true);
    }
    if appeal {
        e = e.field(
            "Contestation",
            "Tu estimes cette décision injustifiée ? Tu peux la contester via la commande `/appeal`.",
            false,
        );
    }
    e
}

// ── Helpers pour les fields ──

/// Couleur selon la gravite d'un avertissement.
pub fn gravity_color(gravity: &str) -> u32 {
    match gravity {
        "low" => COLOR_WARNING,
        "medium" => COLOR_MODERATE,
        "high" | "critical" => COLOR_DANGER,
        _ => COLOR_WARNING,
    }
}

/// Emoji selon la gravite.
pub fn gravity_emoji(gravity: &str) -> &'static str {
    match gravity {
        "low" => "🟡",
        "medium" => "🟠",
        "high" => "🔴",
        "critical" => "⛔",
        _ => "🟡",
    }
}

/// Couleur selon le type d'action de moderation.
pub fn action_color(action: &str) -> u32 {
    match action {
        "warn" => COLOR_WARNING,
        "delete" => COLOR_MODERATE,
        "mute" | "mute_temp" | "mute_permanent" => COLOR_MODERATE,
        "kick" => COLOR_DANGER,
        "ban" | "ban_temp" => COLOR_DANGER,
        "ban_permanent" => COLOR_CRITICAL,
        "unmute" | "unban" => COLOR_SUCCESS,
        _ => COLOR_NEUTRAL,
    }
}

/// Emoji selon le type d'action.
pub fn action_emoji(action: &str) -> &'static str {
    match action {
        "warn" => "⚠\u{fe0f}",
        "delete" => "🗑\u{fe0f}",
        "mute" | "mute_temp" | "mute_permanent" => "🔇",
        "kick" => "👢",
        "ban" | "ban_temp" | "ban_permanent" => "🔨",
        "unmute" => "🔊",
        "unban" => "✅",
        _ => "📝",
    }
}
