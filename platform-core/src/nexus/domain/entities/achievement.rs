//! Hauts faits Nexus : catalogue, liaisons d'identite de jeu et attributions.
//!
//! Le domaine reste independant du jeu : Palworld est le premier adaptateur,
//! mais rien ici ne lui est propre hormis la validation du format d'identite
//! (`GameIdentity`), qui est justement le point ou chaque jeu differe.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::nexus::domain::errors::DomainError;

/// Comment un haut fait peut etre attribue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verification {
    /// Attribuable par un evenement verifie (identite de jeu liee).
    Auto,
    /// Exige une validation d'administrateur, tracee par `granted_by`.
    Manual,
}

impl Verification {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            other => Err(DomainError::Infrastructure(format!(
                "mode de verification inconnu : {other}"
            ))),
        }
    }
}

/// Definition d'un haut fait (catalogue).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: Uuid,
    /// `None` = haut fait transverse (Discord / Nexus).
    pub game: Option<String>,
    pub code: String,
    pub name: String,
    pub description: String,
    pub category: String,
    /// Image choisie par l'administrateur depuis le dashboard.
    pub icon_url: Option<String>,
    pub criteria: serde_json::Value,
    pub verification: Verification,
    pub hidden: bool,
    pub enabled: bool,
}

/// Attribution d'un haut fait a un membre.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAchievement {
    pub id: Uuid,
    pub guild_id: String,
    pub discord_user_id: String,
    pub achievement_id: Uuid,
    pub game_player_id: Option<String>,
    pub source_event_id: Option<String>,
    pub granted_by: Option<String>,
    pub unlocked_at: DateTime<Utc>,
}

/// Liaison entre un membre Discord et son identite dans un jeu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePlayerLink {
    pub id: Uuid,
    pub guild_id: String,
    pub discord_user_id: String,
    pub game: String,
    pub game_player_id: String,
    /// `None` tant que la liaison n'est pas confirmee. Sans elle, aucun haut
    /// fait ne peut etre attribue (fail closed).
    pub verified_at: Option<DateTime<Utc>>,
}

impl GamePlayerLink {
    pub fn is_verified(&self) -> bool {
        self.verified_at.is_some()
    }
}

/// Identite de jeu validee, prete a etre persistee.
///
/// Le type n'existe que construit par [`GameIdentity::parse`] : impossible de
/// passer une chaine arbitraire au repository sans etre passe par la
/// validation du format propre au jeu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameIdentity {
    game: String,
    player_id: String,
}

impl GameIdentity {
    pub fn game(&self) -> &str {
        &self.game
    }

    pub fn player_id(&self) -> &str {
        &self.player_id
    }

    /// Valide le couple (jeu, identifiant joueur).
    ///
    /// Palworld s'appuie sur Steam : l'identifiant attendu est un SteamID64,
    /// soit 17 chiffres commencant par `7656119`. Rejeter tot evite d'ecrire
    /// en base une identite qui ne pourra jamais correspondre a un joueur, et
    /// donne a l'utilisateur un message utile plutot qu'un echec silencieux.
    ///
    /// Les jeux sans format connu acceptent un identifiant libre borne : le
    /// contrat reste ouvert aux futurs adaptateurs (Zomboid, V Rising...) sans
    /// relacher la validation la ou elle est connue.
    pub fn parse(game: &str, player_id: &str) -> Result<Self, DomainError> {
        let game = game.trim().to_ascii_lowercase();
        if game.is_empty() {
            return Err(DomainError::ValidationError("jeu manquant".into()));
        }
        let player_id = player_id.trim();
        if player_id.is_empty() {
            return Err(DomainError::ValidationError(
                "identifiant de joueur manquant".into(),
            ));
        }

        match game.as_str() {
            "palworld" => {
                if !is_steam_id64(player_id) {
                    return Err(DomainError::ValidationError(
                        "SteamID64 invalide : 17 chiffres commencant par 7656119 sont attendus \
                         (Steam > Profil > URL, ou steamid.io)"
                            .into(),
                    ));
                }
            }
            _ => {
                let format_ok = (2..=64).contains(&player_id.len())
                    && player_id
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
                if !format_ok {
                    return Err(DomainError::ValidationError(
                        "identifiant de joueur invalide : 2 a 64 caracteres alphanumeriques, \
                         tiret, point ou souligne"
                            .into(),
                    ));
                }
            }
        }

        Ok(Self {
            game,
            player_id: player_id.to_owned(),
        })
    }
}

/// SteamID64 : 17 chiffres, prefixe `7656119` (plage des comptes individuels).
fn is_steam_id64(value: &str) -> bool {
    value.len() == 17 && value.chars().all(|c| c.is_ascii_digit()) && value.starts_with("7656119")
}

/// Un haut fait tel qu'affiche a un membre : la definition + la date de
/// deblocage quand il la possede.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementProgress {
    pub achievement: Achievement,
    pub unlocked_at: Option<DateTime<Utc>>,
}

impl AchievementProgress {
    pub fn is_unlocked(&self) -> bool {
        self.unlocked_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palworld_exige_un_steam_id64() {
        let identity = GameIdentity::parse("palworld", "76561198000000000").unwrap();
        assert_eq!(identity.game(), "palworld");
        assert_eq!(identity.player_id(), "76561198000000000");
    }

    #[test]
    fn palworld_refuse_un_pseudo_ou_un_id_mal_forme() {
        // Un pseudo n'est pas une identite verifiable : c'est precisement ce
        // que le document interdit d'accepter comme preuve.
        assert!(GameIdentity::parse("palworld", "DarkPoney").is_err());
        // Bon prefixe mais trop court.
        assert!(GameIdentity::parse("palworld", "7656119").is_err());
        // Bonne longueur mais mauvais prefixe.
        assert!(GameIdentity::parse("palworld", "12345678901234567").is_err());
        // Chiffres attendus.
        assert!(GameIdentity::parse("palworld", "7656119800000000x").is_err());
    }

    #[test]
    fn le_jeu_est_normalise_et_l_identifiant_deborde_des_espaces() {
        let identity = GameIdentity::parse("  PalWorld ", " 76561198000000000 ").unwrap();
        assert_eq!(identity.game(), "palworld");
        assert_eq!(identity.player_id(), "76561198000000000");
    }

    #[test]
    fn un_jeu_sans_format_connu_reste_accepte_mais_borne() {
        assert!(GameIdentity::parse("zomboid", "Survivant_01").is_ok());
        assert!(GameIdentity::parse("zomboid", "a").is_err());
        assert!(GameIdentity::parse("zomboid", "avec espace").is_err());
    }

    #[test]
    fn jeu_ou_identifiant_vide_refuse() {
        assert!(GameIdentity::parse("", "76561198000000000").is_err());
        assert!(GameIdentity::parse("palworld", "   ").is_err());
    }
}
