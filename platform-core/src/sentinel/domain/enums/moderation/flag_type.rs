use serde::Deserialize;
use serde::Serialize;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagType {
    Spam,
    /// Insulte CIBLEE : adressee a quelqu'un, ou terme degradant.
    Insult,
    /// Juron d'exclamation : « putain », « merde », « bordel ».
    ///
    /// Flag distinct d'`Insult` et non simple nuance de poids : le
    /// back-office regle un poids par flag, et c'est precisement le fait de
    /// pouvoir les regler separement qui est recherche. En francais ces mots
    /// ponctuent une phrase sans viser personne — les compter comme une
    /// insulte faisait supprimer « merde j'ai oublie ».
    Profanity,
    Link,
    Phishing,
    // IA Vision
    Nsfw,
    Illicit,
    // IA Text Sentiment
    Anger,
    Rage,
    Threat,
    Harassment,
}

impl FlagType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlagType::Spam => "spam",
            FlagType::Insult => "insult",
            FlagType::Profanity => "profanity",
            FlagType::Link => "link",
            FlagType::Phishing => "phishing",
            FlagType::Nsfw => "nsfw",
            FlagType::Illicit => "illicit",
            FlagType::Anger => "anger",
            FlagType::Rage => "rage",
            FlagType::Threat => "threat",
            FlagType::Harassment => "harassment",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "spam" => FlagType::Spam,
            "insult" => FlagType::Insult,
            "profanity" => FlagType::Profanity,
            "link" => FlagType::Link,
            "phishing" => FlagType::Phishing,
            "nsfw" => FlagType::Nsfw,
            "illicit" => FlagType::Illicit,
            "anger" => FlagType::Anger,
            "rage" => FlagType::Rage,
            "threat" => FlagType::Threat,
            "harassment" => FlagType::Harassment,
            _ => FlagType::Spam,
        }
    }
}

#[cfg(test)]
#[path = "tests/flag_type.rs"]
mod tests;
