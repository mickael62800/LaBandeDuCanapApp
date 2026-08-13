use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Statuts possibles d'une idee proposee par un membre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeaStatus {
    /// Vient d'etre proposee, personne ne s'en est encore saisi.
    Nouvelle,
    /// Le staff en discute avec l'auteur dans le salon dedie.
    EnDiscussion,
    /// Retenue par le staff.
    Acceptee,
    /// Ecartee par le staff.
    Refusee,
    /// Acceptee puis effectivement mise en place.
    Realisee,
}

impl IdeaStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nouvelle => "nouvelle",
            Self::EnDiscussion => "en_discussion",
            Self::Acceptee => "acceptee",
            Self::Refusee => "refusee",
            Self::Realisee => "realisee",
        }
    }

    /// Libelle affichable (Discord et web).
    pub fn label(&self) -> &'static str {
        match self {
            Self::Nouvelle => "Nouvelle",
            Self::EnDiscussion => "En discussion",
            Self::Acceptee => "Acceptee",
            Self::Refusee => "Refusee",
            Self::Realisee => "Realisee",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "nouvelle" => Some(Self::Nouvelle),
            "en_discussion" => Some(Self::EnDiscussion),
            "acceptee" => Some(Self::Acceptee),
            "refusee" => Some(Self::Refusee),
            "realisee" => Some(Self::Realisee),
            _ => None,
        }
    }

    pub const VALID_VALUES: &'static [&'static str] = &[
        "nouvelle",
        "en_discussion",
        "acceptee",
        "refusee",
        "realisee",
    ];

    /// Une idee est "tranchee" des que le staff a rendu une decision : elle ne
    /// compte plus dans le quota d'idees ouvertes par membre et son salon peut
    /// etre archive.
    pub fn is_decided(&self) -> bool {
        matches!(self, Self::Acceptee | Self::Refusee | Self::Realisee)
    }

    /// Garde de transition (machine a etats).
    ///
    /// Regles :
    /// - `Realisee` n'a de sens qu'apres une acceptation : on ne saute pas
    ///   directement de `Nouvelle`/`Refusee` a `Realisee` ;
    /// - une idee `Realisee` est terminale : plus aucune transition ;
    /// - sinon tout est libre (le staff peut revenir sur une decision, y
    ///   compris rouvrir une idee refusee).
    pub fn can_transition(from: Self, to: Self) -> bool {
        use IdeaStatus::*;
        match (from, to) {
            // Terminal : une idee realisee ne bouge plus.
            (Realisee, _) => false,
            // "Realisee" suppose une acceptation prealable.
            (_, Realisee) => from == Acceptee,
            _ => true,
        }
    }
}

impl fmt::Display for IdeaStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
