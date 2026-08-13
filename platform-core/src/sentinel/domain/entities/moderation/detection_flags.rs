use crate::sentinel::domain::enums::moderation::flag_type::FlagType;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionFlags {
    pub spam: bool,
    /// Insulte CIBLEE uniquement.
    pub insult: bool,
    /// Juron d'exclamation. `default` : les charges utiles anterieures a la
    /// separation n'ont pas ce champ et doivent rester lisibles.
    #[serde(default)]
    pub profanity: bool,
    pub link: bool,
    #[serde(default)]
    pub phishing: bool,
}

impl DetectionFlags {
    pub fn active_flags(&self) -> Vec<FlagType> {
        let mut flags = Vec::new();
        if self.spam {
            flags.push(FlagType::Spam);
        }
        if self.insult {
            flags.push(FlagType::Insult);
        }
        if self.profanity {
            flags.push(FlagType::Profanity);
        }
        if self.link {
            flags.push(FlagType::Link);
        }
        if self.phishing {
            flags.push(FlagType::Phishing);
        }
        flags
    }
}

#[cfg(test)]
#[path = "tests/detection_flags.rs"]
mod tests;
