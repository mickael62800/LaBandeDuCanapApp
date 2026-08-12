//! Port outbound : publication d'evenements applicatifs vers les autres
//! composants Nexus (typiquement le bot Discord).
//!
//! L'API publie ; le bot consomme. L'implementation concrete (stream Redis,
//! noop en dev) vit dans les adapters de `nexus-api`. Le domaine ne connait
//! que ce trait.

use async_trait::async_trait;

/// Nom des evenements game-portal, partages entre publieur (API) et
/// consommateur (bot). Une constante par event pour eviter les typos.
pub mod game_events {
    /// Ouverture programmee : le conteneur n'est pas encore lance, mais le bot
    /// doit deja creer les salons et le panneau d'inscription (comme pour un
    /// demarrage), afin d'ouvrir les inscriptions a l'avance.
    pub const SERVER_SCHEDULED: &str = "game_server_scheduled";
    pub const SERVER_STARTED: &str = "game_server_started";
    pub const SERVER_STOPPED: &str = "game_server_stopped";
    pub const SERVER_DELETED: &str = "game_server_deleted";
    pub const IP_REVEAL: &str = "game_ip_reveal";
    pub const DAILY_PING: &str = "game_daily_ping";
    pub const GAMES_PANEL_DEPLOY: &str = "games_panel_deploy";
    pub const GAMES_ROLES_ENSURE: &str = "games_roles_ensure";
    pub const GAME_ROLE_DELETE: &str = "game_role_delete";
}

#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publie un evenement. Best-effort : un echec de publication ne doit
    /// jamais faire echouer le cas d'usage metier appelant (le serveur de jeu
    /// a bien demarre meme si le bot n'a pas ete prevenu). Les erreurs sont
    /// loggees par l'implementation.
    async fn publish(&self, event: &str, data: serde_json::Value);
}
