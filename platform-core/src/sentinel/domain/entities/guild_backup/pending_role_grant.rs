//! Entite PURE d'une re-attribution de roles en attente (`pending_role_grant`).
//!
//! A la restauration d'un `GuildSnapshot`, les membres ABSENTS ne peuvent pas
//! recevoir leurs roles immediatement. On persiste alors, pour chaque membre,
//! la liste des NOUVEAUX identifiants de roles (deja remappes old->new cote
//! bot) a lui re-attribuer lorsqu'il REJOINDRA le serveur.
//!
//! Aucune dependance infra (uniquement `serde` pour le contrat bot <-> api).

use serde::{Deserialize, Serialize};

/// Roles a re-attribuer a un membre lors de son retour sur le serveur.
///
/// `role_ids` contient les identifiants Discord des NOUVEAUX roles (ceux du
/// serveur restaure), pas les `old_id` de la capture : la traduction old->new
/// est faite par le bot au moment du restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRoleGrant {
    /// ID Discord du serveur concerne.
    pub guild_id: String,
    /// ID Discord du membre a re-roler a son retour.
    pub user_id: String,
    /// Nouveaux identifiants de roles a lui attribuer.
    pub role_ids: Vec<String>,
}
