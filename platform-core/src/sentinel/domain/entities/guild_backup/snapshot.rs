//! Entites PURES de la sauvegarde de serveur.
//!
//! Aucune dependance infra (pas de sqlx / axum) : uniquement `serde` pour le
//! contrat de serialisation partage bot <-> api. Tous les identifiants Discord
//! d'origine sont conserves sous forme de chaines (`old_id`) pour permettre le
//! remapping a la restauration.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Version du schema de capture. Incrementer si la forme d'un `GuildSnapshot`
/// change de facon non retro-compatible (permet au bot de refuser / migrer un
/// ancien snapshot a la restauration).
pub const SCHEMA_VERSION: u32 = 1;

/// Capture complete de la structure d'un serveur Discord a un instant donne.
///
/// C'est la racine du contrat : le bot la produit a la capture et la consomme
/// a la restauration ; l'API la stocke telle quelle en JSONB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GuildSnapshot {
    /// ID Discord du serveur capture (source).
    pub guild_id: String,
    pub meta: SnapshotMeta,
    pub settings: GuildSettings,
    pub roles: Vec<SnapshotRole>,
    pub categories: Vec<SnapshotCategory>,
    pub channels: Vec<SnapshotChannel>,
    pub bans: Vec<SnapshotBan>,
    pub emojis: Vec<SnapshotEmoji>,
    /// Mapping `user_id -> liste d'old_role_id` (roles a re-attribuer aux
    /// membres presents a la restauration). Cle triee pour un JSON stable.
    #[serde(default)]
    pub member_roles: BTreeMap<String, Vec<String>>,
}

/// Metadonnees de la capture (libelle, horodatage, auteur, version de schema).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotMeta {
    /// Libelle humain de la sauvegarde (ex: "Avant refonte des salons").
    pub label: String,
    /// Date de creation au format RFC3339 (ex: "2026-07-07T12:34:56Z").
    pub created_at: String,
    /// ID Discord de l'auteur de la capture (None = automatique / systeme).
    #[serde(default)]
    pub created_by: Option<String>,
    /// Version du schema de capture (cf. [`SCHEMA_VERSION`]).
    pub schema_version: u32,
}

/// Reglages generaux du serveur.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GuildSettings {
    pub name: String,
    /// Reference / hash de l'icone (image stockee hors JSON, cf. `image_ref`).
    #[serde(default)]
    pub icon: Option<String>,
    /// Niveau de verification Discord (0-4).
    pub verification_level: u32,
    /// Notifications par defaut (0 = tous les messages, 1 = mentions seules).
    pub default_notifications: u32,
    /// Filtre de contenu explicite (0-2).
    pub explicit_content_filter: u32,
    /// Salon AFK d'origine (remappe a la restauration).
    #[serde(default)]
    pub afk_channel_old_id: Option<String>,
    /// Timeout AFK en secondes.
    pub afk_timeout: u32,
    /// Salon systeme d'origine (remappe a la restauration).
    #[serde(default)]
    pub system_channel_old_id: Option<String>,
    /// Permissions de base de `@everyone`, en bitfield textuel.
    ///
    /// Ce role est exclu de la liste des roles — il ne se recree pas, il
    /// existe deja sur le serveur cible. Ses permissions n'etaient donc
    /// sauvegardees NULLE PART, alors qu'elles definissent ce que tout membre
    /// peut faire par defaut : ecrire, se connecter en vocal, mentionner
    /// @everyone. Une restauration laissait les valeurs par defaut de Discord
    /// a la place de la configuration d'origine.
    ///
    /// `default` : les sauvegardes prises avant ce champ ne l'ont pas. Vide =
    /// on n'y touche pas a la restauration, plutot que d'imposer un bitfield
    /// nul qui retirerait tout a tout le monde.
    #[serde(default)]
    pub everyone_permissions: String,
}

/// Un role du serveur.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotRole {
    /// ID Discord d'origine du role (pour remapping des overwrites / membres).
    pub old_id: String,
    pub name: String,
    /// Couleur RGB encodee (0xRRGGBB).
    pub color: u32,
    /// Bitfield des permissions, encode en chaine (les permissions Discord
    /// depassent u32 ; on garde la representation textuelle d'origine).
    pub permissions: String,
    /// Role affiche separement dans la liste des membres.
    pub hoist: bool,
    pub mentionable: bool,
    /// Position dans la hierarchie (plus grand = plus haut).
    pub position: i32,
}

/// Une categorie (conteneur de salons).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotCategory {
    /// ID Discord d'origine de la categorie (parent des salons).
    pub old_id: String,
    pub name: String,
    pub position: i32,
}

/// Un salon (texte / vocal / forum / annonce / stage).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotChannel {
    /// ID Discord d'origine du salon.
    pub old_id: String,
    /// Type de salon : "text" | "voice" | "forum" | "announcement" | "stage".
    pub kind: String,
    pub name: String,
    /// Categorie parente d'origine (None = salon a la racine).
    #[serde(default)]
    pub parent_old_id: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    pub nsfw: bool,
    /// Slowmode en secondes (0 = desactive).
    pub slowmode: u32,
    /// Debit audio en bps (salons vocaux uniquement).
    #[serde(default)]
    pub bitrate: Option<u32>,
    /// Limite d'utilisateurs (salons vocaux uniquement).
    #[serde(default)]
    pub user_limit: Option<u32>,
    pub position: i32,
    /// Permissions specifiques (overwrites) du salon.
    #[serde(default)]
    pub overwrites: Vec<SnapshotOverwrite>,
}

/// Une permission specifique (overwrite) posee sur un salon, ciblant un role
/// ou un membre.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotOverwrite {
    /// ID Discord d'origine de la cible (role ou membre).
    pub target_old_id: String,
    /// "role" | "member".
    pub target_type: String,
    /// Bitfield des permissions autorisees (chaine, cf. [`SnapshotRole`]).
    pub allow: String,
    /// Bitfield des permissions refusees (chaine).
    pub deny: String,
}

/// Un bannissement (conserve pour re-appliquer sur le serveur restaure).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotBan {
    pub user_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Un emoji personnalise. L'image est stockee PAR REFERENCE (pas inline dans le
/// JSON) pour ne pas gonfler le payload : `image_ref` pointe vers l'asset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotEmoji {
    pub name: String,
    /// Reference vers l'image (hash / cle de stockage), pas les octets.
    pub image_ref: String,
}

impl GuildSnapshot {
    /// Nombre de roles captures (pour les resumes).
    pub fn role_count(&self) -> usize {
        self.roles.len()
    }

    /// Nombre de salons captures (categories exclues).
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une sauvegarde prise AVANT l'ajout du champ doit rester lisible.
    ///
    /// Sans ce test, ajouter un champ sans `default` rendrait illisibles
    /// toutes les sauvegardes deja en base — decouvert au moment ou on en a
    /// besoin, c'est-a-dire au pire moment.
    #[test]
    fn ancienne_sauvegarde_sans_permissions_everyone_reste_lisible() {
        let brut = r#"{
            "name": "Mon serveur",
            "verification_level": 1,
            "default_notifications": 0,
            "explicit_content_filter": 2,
            "afk_timeout": 300
        }"#;
        let s: GuildSettings = serde_json::from_str(brut).expect("ancien format illisible");
        assert_eq!(s.name, "Mon serveur");
        // Vide = « inconnu », pas « aucune permission ». La restauration ne
        // touchera pas a @everyone plutot que de tout lui retirer.
        assert!(s.everyone_permissions.is_empty());
    }

    #[test]
    fn permissions_everyone_font_l_aller_retour() {
        let s = GuildSettings {
            name: "S".into(),
            icon: None,
            verification_level: 0,
            default_notifications: 0,
            explicit_content_filter: 0,
            afk_channel_old_id: None,
            afk_timeout: 300,
            system_channel_old_id: None,
            everyone_permissions: "137411140374081".into(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let relu: GuildSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(relu, s);
    }
}
