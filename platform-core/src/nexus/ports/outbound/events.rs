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

    /// Les noms des salons d'une session ont change : le bot doit renommer les
    /// salons DEJA CREES.
    ///
    /// Sans cet evenement, un changement de nom n'aurait pris effet qu'a la
    /// prochaine session — les salons existants auraient garde l'ancien nom
    /// sans que rien ne l'explique.
    pub const SESSION_CHANNELS_RENAMED: &str = "game_session_channels_renamed";

    /// Une session attend toujours son annonce : le bot doit reprendre la
    /// sequence. Emis par la reprise periodique, jamais a l'ouverture.
    pub const SESSION_ANNOUNCEMENT_RETRY: &str = "game_session_announcement_retry";

    /// Charge utile de `SERVER_DELETED`, construite ici pour que publieur et
    /// consommateur ne puissent pas diverger.
    ///
    /// POURQUOI L'EVENEMENT PORTE LES SALONS. Le bot les relisait via l'API.
    /// Mais la fiche est deja soft-deleted quand l'evenement arrive, et la
    /// lecture filtre `deleted_at IS NULL` : le bot recevait un 404 et
    /// renoncait en silence. Les salons Discord d'un jeu supprime survivaient
    /// donc au jeu. Les identifiants sont lus avant la suppression et voyagent
    /// avec le message : il n'y a plus rien a relire, donc plus de course.
    ///
    /// A ne pas confondre avec l'arret : arreter un serveur CONSERVE ses
    /// salons, seule la suppression les emporte.
    pub fn payload_serveur_supprime(
        server_id: &str,
        guild_id: &str,
        text_channel_id: Option<&str>,
        voice_channel_id: Option<&str>,
        template_id: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "server_id": server_id,
            "guild_id": guild_id,
            "text_channel_id": text_channel_id,
            "voice_channel_id": voice_channel_id,
            "template_id": template_id,
        })
    }
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
