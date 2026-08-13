//! Entites des "evenements de serveur" Game Portal : reglages par
//! (guild, template) et inscriptions des joueurs a une session.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Reglages d'un template pour une guild donnee (le catalogue de templates
/// est global ; le role a pinguer est propre a chaque serveur Discord).
#[derive(Debug, Clone)]
pub struct GameTemplateSettings {
    pub guild_id: String,
    pub template_slug: String,
    /// Role Discord a pinguer pour ce jeu sur cette guild.
    pub discord_role_id: Option<String>,
}

/// Inscription d'un joueur a une session (bouton "Je m'inscris").
#[derive(Debug, Clone)]
pub struct GameSessionRegistration {
    pub id: Uuid,
    pub server_id: Uuid,
    pub user_id: String,
    pub registered_at: DateTime<Utc>,
}
