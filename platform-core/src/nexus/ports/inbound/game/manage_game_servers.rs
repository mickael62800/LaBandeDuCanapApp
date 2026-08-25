//! Use case Game Servers — interface utilise'e par les handlers HTTP et le worker.

use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

use crate::nexus::domain::entities::game::server::{CreateGameServerCommand, GameServer};
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::outbound::game::container_runtime::ContainerStats;

#[derive(Debug, Clone)]
pub struct GameServerDetail {
    pub server: GameServer,
    pub config: HashMap<String, String>,
}

/// Résultat d'une demande d'ouverture via le bouton « Révéler l'adresse IP ».
/// Le serveur a été démarré si nécessaire et la révélation est programmée à
/// `reveal_at`. Le bot s'en sert pour annoncer le décompte dans le panneau.
#[derive(Debug, Clone)]
pub struct RequestIpRevealOutcome {
    /// Délai retenu avant la révélation, lu dans la config de la guilde.
    pub delay_minutes: i64,
    /// Heure de révélation programmée (`now + delay_minutes`).
    pub reveal_at: chrono::DateTime<chrono::Utc>,
    /// `true` si un démarrage doit être lancé (le serveur était à l'arrêt).
    /// Le use case ne démarre PAS lui-même — le pull d'image bloquerait la
    /// requête ; le handler lance `start` en tâche de fond quand ce drapeau
    /// est vrai.
    pub started: bool,
}

#[async_trait]
pub trait ManageGameServersUseCase: Send + Sync {
    // ── CRUD basique ──────────────────────────────────────────────────
    async fn create(&self, cmd: CreateGameServerCommand) -> Result<GameServer, DomainError>;
    async fn list_for_guild(&self, guild_id: &str) -> Result<Vec<GameServer>, DomainError>;
    async fn get(&self, id: Uuid) -> Result<GameServerDetail, DomainError>;

    /// Soft-delete : stop container + remove + remove volume + soft-delete DB.
    async fn delete(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError>;

    // ── Lifecycle Docker ──────────────────────────────────────────────
    async fn start(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError>;
    async fn stop(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError>;
    async fn restart(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError>;

    /// Rend immédiatement l'adresse publique, avant l'échéance programmée.
    /// Réservé aux appels d'administration par la couche HTTP.
    async fn reveal_ip(&self, id: Uuid, actor_user_id: &str) -> Result<(), DomainError>;

    /// Demande d'ouverture depuis le bouton « Révéler l'adresse IP » du panneau.
    ///
    /// Contrairement à `reveal_ip` (révélation immédiate, réservée à l'admin),
    /// ce flux DÉMARRE le serveur s'il est à l'arrêt puis PROGRAMME la
    /// révélation à `now + reveal_delay_minutes` (config de la guilde, défaut
    /// 10 min). Le worker `reveal-ip` publie l'adresse à l'échéance, une fois le
    /// conteneur `running`. Échoue en fermeture si l'hôte public n'est pas
    /// configuré ou si l'adresse est déjà révélée.
    async fn request_ip_reveal(
        &self,
        id: Uuid,
        actor_user_id: &str,
    ) -> Result<RequestIpRevealOutcome, DomainError>;

    /// Mode « Préparation » : programme l'ouverture à `reveal_at` sans démarrer
    /// le conteneur. Le serveur passe en `scheduled` ; le worker le démarrera
    /// ~5 min avant l'heure, et l'IP sera révélée à l'heure dite. Les salons
    /// Discord et le panneau d'inscription sont créés dès maintenant (par le
    /// bot, sur l'événement `game_server_scheduled` publié par la couche HTTP).
    /// Programme l'ouverture, et l'heure de fin annoncee quand elle est
    /// connue. Sans cette derniere, un conteneur arrete ne peut pas etre
    /// distingue d'une session terminee (cf. `session_state`).
    async fn schedule(
        &self,
        id: Uuid,
        reveal_at: chrono::DateTime<chrono::Utc>,
        closes_at: Option<chrono::DateTime<chrono::Utc>>,
        actor_user_id: &str,
    ) -> Result<(), DomainError>;

    /// Définit (ou efface) l'heure de révélation programmée de l'IP sans changer
    /// l'état du conteneur. Utilisé par « Lancer maintenant » pour programmer en
    /// plus une révélation automatique, ou pour l'ajuster/annuler.
    async fn set_reveal_schedule(
        &self,
        id: Uuid,
        reveal_at: Option<chrono::DateTime<chrono::Utc>>,
        actor_user_id: &str,
    ) -> Result<(), DomainError>;

    // ── Observabilite ─────────────────────────────────────────────────
    async fn get_logs(&self, id: Uuid, lines: u32) -> Result<Vec<String>, DomainError>;
    async fn get_stats(&self, id: Uuid) -> Result<ContainerStats, DomainError>;

    // ── Config ────────────────────────────────────────────────────────
    /// Update full set d'overrides — atomique. Owner du serveur ou Admin+.
    async fn update_config(
        &self,
        id: Uuid,
        entries: HashMap<String, String>,
        actor_user_id: &str,
    ) -> Result<(), DomainError>;

    /// Ajuste la memoire et les coeurs alloues a un serveur.
    ///
    /// Les valeurs sont validees contre les bornes du modele de jeu : une
    /// memoire sous le minimum donne un serveur qui plante au demarrage, et
    /// c'est le genre de reglage qu'on rate en se trompant d'unite.
    ///
    /// L'effet est differe a la prochaine reconstruction du conteneur : Docker
    /// fige ces limites a la creation.
    async fn update_resources(
        &self,
        id: Uuid,
        memory_mb: i32,
        cpu_limit: Option<f64>,
        actor_user_id: &str,
    ) -> Result<(), DomainError>;

    /// Noms libres des salons Discord de ce serveur.
    ///
    /// `None` efface le nom libre et rend la main au modele de la guilde ;
    /// c'est une valeur voulue, pas une absence de changement. Les trois sont
    /// donc toujours transmis ensemble, et l'appelant envoie l'etat complet du
    /// formulaire.
    async fn update_channel_names(
        &self,
        id: Uuid,
        registration: Option<String>,
        private: Option<String>,
        voice: Option<String>,
        actor_user_id: &str,
    ) -> Result<(), DomainError>;

    // ── Console RCON ──────────────────────────────────────────────────
    /// Execute une commande RCON (Owner uniquement). Retourne la reponse brute.
    async fn execute_rcon(
        &self,
        id: Uuid,
        command: &str,
        actor_user_id: &str,
    ) -> Result<String, DomainError>;

    // ── Catalogue de commandes ────────────────────────────────────────
    /// Commandes d'administration proposees pour ce serveur, telles que
    /// decrites par son modele de jeu. Vide si le jeu n'en declare aucune.
    async fn list_commands(
        &self,
        id: Uuid,
    ) -> Result<Vec<crate::nexus::domain::entities::game::command::GameCommand>, DomainError>;

    /// Execute une commande DU CATALOGUE, designee par sa cle.
    ///
    /// Le gabarit est retrouve cote serveur et compose a partir des parametres
    /// valides : le navigateur n'envoie jamais de commande, seulement une cle
    /// et des valeurs. Sans cela, un bouton « bannir » serait une console RCON
    /// ouverte a quiconque sait forger une requete.
    async fn run_catalog_command(
        &self,
        id: Uuid,
        command_key: &str,
        params: &[(String, String)],
        actor_user_id: &str,
    ) -> Result<String, DomainError>;

    /// Joueurs actuellement connectes, lus par la commande RCON propre au jeu.
    async fn list_online_players(
        &self,
        id: Uuid,
        actor_user_id: &str,
    ) -> Result<Vec<crate::nexus::domain::entities::game::presence::PlayerPresence>, DomainError>;
}
