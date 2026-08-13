use serde::Deserialize;
use serde::Serialize;
/// Phase 2 A.3 — Gravite d'une action de moderation.
///
/// Lie au type Postgres `moderation_gravity` (migration 103).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModerationGravity {
    Low,
    Medium,
    High,
    Critical,
}

impl ModerationGravity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "tests/moderation_gravity.rs"]
mod tests;
