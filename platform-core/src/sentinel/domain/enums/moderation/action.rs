use serde::Deserialize;
use serde::Serialize;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    None,
    Warn,
    Delete,
    Mute,
    Kick,
    Ban,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::None => "none",
            Action::Warn => "warn",
            Action::Delete => "delete",
            Action::Mute => "mute",
            Action::Kick => "kick",
            Action::Ban => "ban",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "warn" => Action::Warn,
            "delete" => Action::Delete,
            "mute" => Action::Mute,
            "kick" => Action::Kick,
            "ban" => Action::Ban,
            _ => Action::None,
        }
    }
}

#[cfg(test)]
#[path = "tests/action.rs"]
mod tests;
