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
    /// Demande au bot l'inventaire reel de la guilde (roles et panneaux
    /// vivants). Lui seul voit Discord : sans cette photographie, l'API ne peut
    /// constater aucune divergence.
    pub const GAMES_SYNC_REQUESTED: &str = "games_sync_requested";
    /// Un redemarrage programme approche. Double annonce voulue : le message
    /// RCON touche ceux qui JOUENT, celui-ci touche ceux qui vont se connecter.
    /// Aucun des deux ne remplace l'autre.
    pub const SERVER_RESTART_WARNING: &str = "game_server_restart_warning";
    /// Le redemarrage programme est termine, le serveur est de nouveau la.
    pub const SERVER_RESTARTED: &str = "game_server_restarted";
}

/// Coussin Piege.
pub mod coussin_events {
    /// Une fouille vient d'etre tranchee faute de reaction de la victime. Le
    /// bot en fait le recit dans le salon d'origine : sans cela, la fenetre de
    /// defense se refermerait dans le silence et personne ne saurait ce qui
    /// s'est passe.
    pub const STEAL_RESOLVED: &str = "coussin_steal_resolved";
}

/// Hauts faits (cf. DOC/Nexus/haut-faits.md).
pub mod achievement_events {
    /// Un haut fait vient d'etre attribue : le bot publie l'annonce.
    /// Emis UNIQUEMENT apres persistance confirmee.
    pub const ACHIEVEMENT_UNLOCKED: &str = "achievement.unlocked";
}

#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publie un evenement. Best-effort : un echec de publication ne doit
    /// jamais faire echouer le cas d'usage metier appelant (le serveur de jeu
    /// a bien demarre meme si le bot n'a pas ete prevenu). Les erreurs sont
    /// loggees par l'implementation.
    async fn publish(&self, event: &str, data: serde_json::Value);
}
// Port de publication des événements NEXUS. Les événements décrivent les
// ouvertures, arrêts, suppressions et révélations d'adresse à destination du bot.
