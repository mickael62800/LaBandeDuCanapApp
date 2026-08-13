use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Types d'actions de moderation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationActionType {
    Warn,
    #[serde(rename = "mute_temp")]
    MuteTemp,
    #[serde(rename = "mute_permanent")]
    MutePermanent,
    Unmute,
    #[serde(rename = "ban_temp")]
    BanTemp,
    #[serde(rename = "ban_permanent")]
    BanPermanent,
    Unban,
    Call,
}

impl ModerationActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::MuteTemp => "mute_temp",
            Self::MutePermanent => "mute_permanent",
            Self::Unmute => "unmute",
            Self::BanTemp => "ban_temp",
            Self::BanPermanent => "ban_permanent",
            Self::Unban => "unban",
            Self::Call => "call",
        }
    }

    /// Parse un type d'action depuis une string. Retourne `None` si invalide.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "warn" => Some(Self::Warn),
            "mute_temp" => Some(Self::MuteTemp),
            "mute_permanent" => Some(Self::MutePermanent),
            "unmute" => Some(Self::Unmute),
            "ban_temp" => Some(Self::BanTemp),
            "ban_permanent" => Some(Self::BanPermanent),
            "unban" => Some(Self::Unban),
            "call" => Some(Self::Call),
            _ => None,
        }
    }

    /// True si c'est un type de ban (temp ou permanent).
    pub fn is_ban(&self) -> bool {
        matches!(self, Self::BanTemp | Self::BanPermanent)
    }

    /// True si c'est un type de mute (temp ou permanent).
    pub fn is_mute(&self) -> bool {
        matches!(self, Self::MuteTemp | Self::MutePermanent)
    }

    /// True si l'action est une sanction temporaire (avec duration).
    /// Utilisee pour decider s'il faut creer un rappel de fin de sanction.
    pub fn is_temporary(&self) -> bool {
        matches!(self, Self::MuteTemp | Self::BanTemp)
    }

    /// Helper pour appelants qui travaillent avec un `&str` au lieu de l'enum
    /// (ex: handlers HTTP avec DTO brut). `false` pour une chaine inconnue.
    pub fn is_temporary_str(action_type: &str) -> bool {
        matches!(action_type, "mute_temp" | "ban_temp")
    }

    /// Liste des valeurs valides.
    pub const VALID_VALUES: &'static [&'static str] = &[
        "warn",
        "mute_temp",
        "mute_permanent",
        "unmute",
        "ban_temp",
        "ban_permanent",
        "unban",
        "call",
    ];
}

impl fmt::Display for ModerationActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
#[path = "tests/moderation_action_type.rs"]
mod tests;
