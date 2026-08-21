//! Entités de l'apaisement (« conflit ») Atrium.
//!
//! Sentinel détecte une tension dans un salon et émet `atrium_calming_requested` ;
//! Atrium y répond par un rappel apaisant. Ce message était figé dans le bot
//! (`match kind`) : il est désormais généré par IA à partir d'un contexte réglable
//! par serveur (`conflict_context`), avec repli sur les rappels statiques
//! historiques quand l'IA est indisponible. Ces rappels restent donc la source de
//! vérité du ton « par défaut » : ils vivent ici, plus dans le bot.

/// Nature de la tension signalée par Sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    Flood,
    Toxicity,
    Phishing,
    Other,
}

impl ConflictKind {
    /// `unsafe_link` est un alias historique de `phishing` côté Sentinel ; tout
    /// libellé inconnu retombe sur `Other` (fail-safe : on apaise quand même).
    pub fn parse(raw: &str) -> Self {
        match raw {
            "flood" => Self::Flood,
            "toxicity" => Self::Toxicity,
            "phishing" | "unsafe_link" => Self::Phishing,
            _ => Self::Other,
        }
    }

    /// Description en clair de la tension, injectée dans le prompt pour guider
    /// le modèle sans lui laisser deviner la situation.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Flood => {
                "des envois trop rapprochés (flood) qui empêchent les autres de participer"
            }
            Self::Toxicity => "des propos agressifs ou blessants (toxicité)",
            Self::Phishing => "un lien suspect ou une tentative d'hameçonnage",
            Self::Other => "une montée de tension dans le salon",
        }
    }

    /// Rappel statique de repli, identique aux messages historiques du bot.
    /// Utilisé quand l'IA est indisponible (ou Atrium désactivé sur le serveur).
    pub fn fallback_message(self) -> &'static str {
        match self {
            Self::Flood => "🕊️ Merci de ralentir un peu : évitez les envois successifs et laissez chacun participer.",
            Self::Toxicity => "🕊️ Restons respectueux. Les attaques personnelles et les propos blessants n'ont pas leur place ici.",
            Self::Phishing => "⚠️ N'ouvrez pas les liens suspects. Signalez-les à la modération et respectez les règles de sécurité.",
            Self::Other => "🕊️ Le ton monte un peu. Merci de prendre une pause, de rester respectueux et de suivre le règlement.",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CalmingRequest {
    /// Guilde Discord dans laquelle la tension a été détectée.
    pub guild_id: String,
    /// Salon où le rappel doit être publié.
    pub channel_id: String,
    /// Catégorie normalisée de tension.
    pub kind: ConflictKind,
    /// Texte libre configuré par les administrateurs (`conflict_context`) pour
    /// ajuster le ton de l'apaisement. Vide = comportement par défaut.
    pub admin_context: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalmingReply {
    pub content: String,
    /// `false` quand le repli statique a été servi (IA indisponible ou désactivée).
    pub generated_by_ai: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CalmingError {
    #[error("{0} est obligatoire")]
    Missing(&'static str),
    #[error("{field} depasse la limite de {limit} caracteres")]
    TooLong { field: &'static str, limit: usize },
}

#[cfg(test)]
mod tests {
    use super::ConflictKind;

    #[test]
    fn parses_known_kinds_and_legacy_alias() {
        assert_eq!(ConflictKind::parse("flood"), ConflictKind::Flood);
        assert_eq!(ConflictKind::parse("toxicity"), ConflictKind::Toxicity);
        assert_eq!(ConflictKind::parse("phishing"), ConflictKind::Phishing);
        assert_eq!(ConflictKind::parse("unsafe_link"), ConflictKind::Phishing);
    }

    #[test]
    fn parses_unknown_kind_as_safe_default() {
        assert_eq!(ConflictKind::parse("unknown"), ConflictKind::Other);
        assert_eq!(ConflictKind::parse(""), ConflictKind::Other);
    }

    #[test]
    fn every_kind_has_a_description_and_fallback() {
        for kind in [
            ConflictKind::Flood,
            ConflictKind::Toxicity,
            ConflictKind::Phishing,
            ConflictKind::Other,
        ] {
            assert!(!kind.describe().is_empty());
            assert!(!kind.fallback_message().is_empty());
        }
    }

    #[test]
    fn all_descriptions_and_fallbacks_are_not_empty() {
        assert!(!ConflictKind::Flood.describe().is_empty());
        assert!(!ConflictKind::Toxicity.describe().is_empty());
        assert!(!ConflictKind::Phishing.describe().is_empty());
        assert!(!ConflictKind::Other.describe().is_empty());

        assert!(!ConflictKind::Flood.fallback_message().is_empty());
        assert!(!ConflictKind::Toxicity.fallback_message().is_empty());
        assert!(!ConflictKind::Phishing.fallback_message().is_empty());
        assert!(!ConflictKind::Other.fallback_message().is_empty());
    }
}
