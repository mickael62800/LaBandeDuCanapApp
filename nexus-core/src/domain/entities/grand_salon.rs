//! Le Grand Salon : jeu social de La Bande du Canapé.
//!
//! Les membres deviennent des habitués, créent des cercles, défendent des
//! motions du salon et font vivre la Gazette du Canapé. Les chiffres restent
//! privés ; les interfaces exposent des paliers narratifs.

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ressource {
    Rayonnement,
    Jetons,
    Reputation,
    BonsPlans,
    Reseau,
}

impl Ressource {
    pub fn key(self) -> &'static str {
        match self {
            Self::Rayonnement => "rayonnement",
            Self::Jetons => "jetons",
            Self::Reputation => "reputation",
            Self::BonsPlans => "bons_plans",
            Self::Reseau => "reseau",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Rayonnement => "Rayonnement",
            Self::Jetons => "Jetons canapé",
            Self::Reputation => "Réputation",
            Self::BonsPlans => "Bons plans",
            Self::Reseau => "Réseau",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ressources {
    pub rayonnement: i64,
    pub jetons: i64,
    pub reputation: i64,
    pub bons_plans: i64,
    pub reseau: i64,
}

impl Ressources {
    pub fn newcomer(starting_jetons: i64) -> Self {
        Self {
            rayonnement: 0,
            jetons: starting_jetons.max(0),
            reputation: 0,
            bons_plans: 0,
            reseau: 0,
        }
    }
    pub fn get(self, resource: Ressource) -> i64 {
        match resource {
            Ressource::Rayonnement => self.rayonnement,
            Ressource::Jetons => self.jetons,
            Ressource::Reputation => self.reputation,
            Ressource::BonsPlans => self.bons_plans,
            Ressource::Reseau => self.reseau,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Habitué {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub display_name: String,
    pub ressources: Ressources,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Palier {
    Discret,
    Connu,
    Installe,
    Incontournable,
    Legendaire,
}

pub fn palier(value: i64) -> Palier {
    match value {
        ..=99 => Palier::Discret,
        100..=499 => Palier::Connu,
        500..=1_999 => Palier::Installe,
        2_000..=9_999 => Palier::Incontournable,
        _ => Palier::Legendaire,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CercleKind {
    Bande,
    Club,
    Collectif,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cercle {
    pub id: Uuid,
    pub guild_id: String,
    pub kind: CercleKind,
    pub name: String,
    pub devise: String,
    pub caisse: i64,
    pub reputation: i64,
    pub rayonnement: i64,
    pub founder_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub dissolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionStatus {
    EnVote,
    Adoptee,
    Rejetee,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionDuSalon {
    pub id: Uuid,
    pub guild_id: String,
    pub titre: String,
    pub texte: String,
    pub status: MotionStatus,
    pub author_id: Uuid,
    pub closes_at: DateTime<Utc>,
    pub soutien_pour: i64,
    pub soutien_contre: i64,
}

impl MotionDuSalon {
    pub fn should_pass(&self, votes_for: i64, votes_against: i64) -> bool {
        votes_for.saturating_add(self.soutien_pour)
            > votes_against.saturating_add(self.soutien_contre)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dossier {
    pub id: Uuid,
    pub guild_id: String,
    pub owner_id: Uuid,
    pub subject: String,
    pub verified: bool,
    pub revealed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GazetteArticle {
    pub id: Uuid,
    pub guild_id: String,
    pub headline: String,
    pub body: String,
    pub published_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newcomer_starts_with_only_configured_jetons() {
        let r = Ressources::newcomer(250);
        assert_eq!(r.jetons, 250);
        assert_eq!(r.get(Ressource::Rayonnement), 0);
        assert_eq!(Ressources::newcomer(-1).jetons, 0);
    }

    #[test]
    fn salon_tiers_have_stable_boundaries() {
        assert_eq!(palier(99), Palier::Discret);
        assert_eq!(palier(100), Palier::Connu);
        assert_eq!(palier(500), Palier::Installe);
        assert_eq!(palier(2_000), Palier::Incontournable);
        assert_eq!(palier(10_000), Palier::Legendaire);
    }

    #[test]
    fn a_motion_needs_a_strict_majority_after_support() {
        let motion = MotionDuSalon {
            id: Uuid::nil(),
            guild_id: "salon".into(),
            titre: "Soirée plaid".into(),
            texte: "".into(),
            status: MotionStatus::EnVote,
            author_id: Uuid::nil(),
            closes_at: Utc::now(),
            soutien_pour: 3,
            soutien_contre: 1,
        };
        assert!(motion.should_pass(8, 9));
        assert!(!motion.should_pass(7, 9));
    }
}
