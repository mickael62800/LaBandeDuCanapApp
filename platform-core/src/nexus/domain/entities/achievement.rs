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

/// Plateforme de compte sur laquelle un joueur est identifie.
///
/// Palworld est jouable via Steam et via le Microsoft Store / Xbox : les deux
/// n'ont pas le meme format d'identifiant, d'ou le besoin de savoir de quelle
/// plateforme parle une liaison avant de la valider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Steam,
    Xbox,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Steam => "steam",
            Self::Xbox => "xbox",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Steam => "Steam",
            Self::Xbox => "Xbox",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "steam" => Ok(Self::Steam),
            "xbox" => Ok(Self::Xbox),
            other => Err(DomainError::ValidationError(format!(
                "plateforme inconnue : {other}"
            ))),
        }
    }
}

/// Liaison entre un membre Discord et son identite dans un jeu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePlayerLink {
    pub id: Uuid,
    pub guild_id: String,
    pub discord_user_id: String,
    pub game: String,
    pub platform: Platform,
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
    platform: Platform,
    player_id: String,
}

impl GameIdentity {
    pub fn game(&self) -> &str {
        &self.game
    }

    pub fn platform(&self) -> Platform {
        self.platform
    }

    pub fn player_id(&self) -> &str {
        &self.player_id
    }

    /// Valide le triplet (jeu, plateforme, identifiant joueur).
    ///
    /// Le format depend de la PLATEFORME, pas du jeu : un meme jeu (Palworld)
    /// se joue via Steam ou via le Microsoft Store. Valider tot evite d'ecrire
    /// en base une identite qui ne pourra jamais correspondre a un joueur, et
    /// donne a l'utilisateur un message utile plutot qu'un echec silencieux.
    ///
    ///  - **Steam** : SteamID64, 17 chiffres commencant par `7656119` ;
    ///  - **Xbox** : XUID (16 chiffres) ou Gamertag. Le Gamertag est accepte
    ///    parce que, contrairement a un nom de personnage choisi librement dans
    ///    le jeu, il identifie de facon unique un compte Microsoft.
    pub fn parse(game: &str, platform: Platform, player_id: &str) -> Result<Self, DomainError> {
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

        match platform {
            Platform::Steam => {
                if !is_steam_id64(player_id) {
                    return Err(DomainError::ValidationError(
                        "SteamID64 invalide : 17 chiffres commencant par 7656119 sont attendus \
                         (Steam > Profil > Details du compte, ou steamid.io)"
                            .into(),
                    ));
                }
            }
            Platform::Xbox => {
                if !(is_xuid(player_id) || is_gamertag(player_id)) {
                    return Err(DomainError::ValidationError(
                        "Identifiant Xbox invalide : XUID (16 chiffres) ou Gamertag \
                         (3 a 15 caracteres) attendus"
                            .into(),
                    ));
                }
            }
        }

        Ok(Self {
            game,
            platform,
            player_id: player_id.to_owned(),
        })
    }
}

/// SteamID64 : 17 chiffres, prefixe `7656119` (plage des comptes individuels).
fn is_steam_id64(value: &str) -> bool {
    value.len() == 17 && value.chars().all(|c| c.is_ascii_digit()) && value.starts_with("7656119")
}

/// XUID Xbox : 16 chiffres. On refuse une suite de zeros, qui est la valeur de
/// remplissage renvoyee par certains serveurs quand l'identite est inconnue.
fn is_xuid(value: &str) -> bool {
    value.len() == 16
        && value.chars().all(|c| c.is_ascii_digit())
        && value.chars().any(|c| c != '0')
}

/// Gamertag Xbox : 3 a 15 caracteres, lettres/chiffres/espaces, avec au moins
/// une lettre — sans quoi une suite de chiffres passerait ici alors qu'elle
/// releve du XUID.
fn is_gamertag(value: &str) -> bool {
    (3..=15).contains(&value.chars().count())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_')
        && value.chars().any(|c| c.is_ascii_alphabetic())
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
    fn steam_exige_un_steam_id64() {
        let identity =
            GameIdentity::parse("palworld", Platform::Steam, "76561198000000000").unwrap();
        assert_eq!(identity.game(), "palworld");
        assert_eq!(identity.platform(), Platform::Steam);
        assert_eq!(identity.player_id(), "76561198000000000");
    }

    #[test]
    fn steam_refuse_un_pseudo_ou_un_id_mal_forme() {
        // Un nom de personnage n'est pas une identite verifiable : c'est
        // precisement ce que le document interdit d'accepter comme preuve.
        let p = Platform::Steam;
        assert!(GameIdentity::parse("palworld", p, "DarkPoney").is_err());
        assert!(GameIdentity::parse("palworld", p, "7656119").is_err());
        assert!(GameIdentity::parse("palworld", p, "12345678901234567").is_err());
        assert!(GameIdentity::parse("palworld", p, "7656119800000000x").is_err());
    }

    #[test]
    fn xbox_accepte_un_xuid_ou_un_gamertag() {
        let p = Platform::Xbox;
        assert!(GameIdentity::parse("palworld", p, "2533274800000000").is_ok());
        assert!(GameIdentity::parse("palworld", p, "DarkPoney").is_ok());
        assert!(GameIdentity::parse("palworld", p, "Dark Poney 42").is_ok());
    }

    #[test]
    fn xbox_refuse_les_valeurs_de_remplissage_et_les_formats_hors_bornes() {
        let p = Platform::Xbox;
        // Suite de zeros : valeur de remplissage, pas une identite.
        assert!(GameIdentity::parse("palworld", p, "0000000000000000").is_err());
        // Trop court / trop long pour un Gamertag.
        assert!(GameIdentity::parse("palworld", p, "ab").is_err());
        assert!(GameIdentity::parse("palworld", p, "GamertagBeaucoupTropLong").is_err());
        // Suite de chiffres qui n'est pas un XUID valide.
        assert!(GameIdentity::parse("palworld", p, "12345").is_err());
    }

    #[test]
    fn le_jeu_est_normalise_et_l_identifiant_deborde_des_espaces() {
        let identity =
            GameIdentity::parse("  PalWorld ", Platform::Steam, " 76561198000000000 ").unwrap();
        assert_eq!(identity.game(), "palworld");
        assert_eq!(identity.player_id(), "76561198000000000");
    }

    #[test]
    fn jeu_ou_identifiant_vide_refuse() {
        assert!(GameIdentity::parse("", Platform::Steam, "76561198000000000").is_err());
        assert!(GameIdentity::parse("palworld", Platform::Steam, "   ").is_err());
    }

    #[test]
    fn la_plateforme_se_lit_depuis_une_chaine() {
        assert_eq!(Platform::parse("steam").unwrap(), Platform::Steam);
        assert_eq!(Platform::parse(" XBOX ").unwrap(), Platform::Xbox);
        assert!(Platform::parse("playstation").is_err());
    }

    #[test]
    fn verification_conversions() {
        assert_eq!(Verification::Auto.as_str(), "auto");
        assert_eq!(Verification::Manual.as_str(), "manual");
        assert_eq!(Verification::parse("auto").unwrap(), Verification::Auto);
        assert_eq!(Verification::parse("manual").unwrap(), Verification::Manual);
        assert!(Verification::parse("invalid").is_err());
    }

    #[test]
    fn platform_conversions() {
        assert_eq!(Platform::Steam.as_str(), "steam");
        assert_eq!(Platform::Xbox.as_str(), "xbox");
        assert_eq!(Platform::Steam.label(), "Steam");
        assert_eq!(Platform::Xbox.label(), "Xbox");
    }

    #[test]
    fn game_player_link_verification() {
        let mut link = GamePlayerLink {
            id: Uuid::nil(),
            guild_id: "123".into(),
            discord_user_id: "456".into(),
            game: "palworld".into(),
            platform: Platform::Steam,
            game_player_id: "76561198000000000".into(),
            verified_at: None,
        };
        assert!(!link.is_verified());
        link.verified_at = Some(Utc::now());
        assert!(link.is_verified());
    }

    #[test]
    fn achievement_progress_unlocked() {
        let ach = Achievement {
            id: Uuid::nil(),
            game: None,
            code: "a".into(),
            name: "a".into(),
            description: "a".into(),
            category: "a".into(),
            icon_url: None,
            criteria: serde_json::json!({}),
            verification: Verification::Auto,
            hidden: false,
            enabled: true,
        };
        let mut prog = AchievementProgress {
            achievement: ach,
            unlocked_at: None,
        };
        assert!(!prog.is_unlocked());
        prog.unlocked_at = Some(Utc::now());
        assert!(prog.is_unlocked());
    }
}
