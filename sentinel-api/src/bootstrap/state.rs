//! `AppState` — la composition root de l'API.
//!
//! # Decoupage par domaine
//!
//! `AppState` etait un objet plat de ~100 champs. C'est desormais un assemblage
//! de **six sous-etats par domaine** (`ai`, `moderation`, `audit`, `community`,
//! `system`, `guild_backup`). Chaque sous-etat implemente `FromRef<AppState>`,
//! ce qui permet a un handler de declarer exactement les ports dont il depend :
//!
//! ```ignore
//! async fn ban(State(st): State<ModerationState>, ...) { ... }
//! ```
//!
//! Un tel handler ne peut plus toucher au systeme ou a l'IA — le compilateur
//! ne lui en donne pas les moyens. C'est l'interet reel du decoupage : rendre
//! les dependances d'un fichier verifiables plutot que declaratives.
//!
//! # Ce qui reste plat, et pourquoi
//!
//! Les champs restants ne sont pas des oublis :
//!
//! - **Infrastructure partagee** (`broadcaster`, `redis_client`, `cache`,
//!   `discord_api`, `job_client`, `log_repo`, `bot_config_repo`, `pg_pool`) :
//!   consommee par plusieurs domaines et par le bootstrap.
//! - **Configuration lue par les middlewares** (`api_key`, `guild_id`,
//!   `superadmin_user_ids`, `metrics_token`, `discord_bot_token`) : les
//!   middlewares sont montes via `from_fn_with_state` au niveau du routeur,
//!   hors de tout domaine.
//! - **`nexus_games`** : relais vers l'autre plateforme, sans domaine ici.
//!
//! Deux fichiers de handlers restent aussi sur `AppState`, faute d'appartenir a
//! un domaine unique : `handlers/moderation/purge.rs` (purge des audit-logs ET
//! des logs systeme) et `handlers/community/voice_channels.rs` (reclame
//! `tickets_uc`, `audit_logs_uc` et `superadmin_user_ids`). Les forcer dans un
//! sous-etat aurait reconstitue un god-object en miniature.
//!
//! Regle : au-dela de 2-3 ports etrangers, c'est le fichier qui est mal range,
//! pas le sous-etat qui est trop etroit.

pub mod ai;
pub mod audit;
pub mod community;
pub mod guild_backup;
pub mod moderation;
pub mod ops;
pub mod system;

pub use ai::AiState;
pub use audit::AuditState;
pub use community::CommunityState;
pub use guild_backup::GuildBackupState;
pub use moderation::ModerationState;
pub use ops::OpsState;
pub use system::SystemState;

use std::sync::Arc;

use crate::adapters::outbound::discord_api::DiscordApi;
use crate::adapters::outbound::job_client::JobClient;
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::adapters::outbound::ws::broadcaster::EventBroadcaster;
use sentinel_core::ports::outbound::system::bot_config_repository::BotConfigRepository;
use ops_core::ports::outbound::log_repository::LogRepository;
#[derive(Clone)]
pub struct AppState {
    // ─────────────────────────────────────────────────────────────────────
    // Sous-etats par domaine. Forme cible : un handler prend `State<XState>`
    // plutot que `State<AppState>`. Cf. la doc du module.
    // ─────────────────────────────────────────────────────────────────────
    pub ai: AiState,
    pub moderation: ModerationState,
    pub audit: AuditState,
    pub community: CommunityState,
    pub ops: OpsState,
    pub system: SystemState,
    pub guild_backup: GuildBackupState,

    // ─────────────────────────────────────────────────────────────────────
    // Champs plats historiques — en cours de retrait, domaine par domaine.
    // Ils pointent sur les memes `Arc` que les sous-etats ci-dessus : les
    // dupliquer ne coute qu'un compteur de reference, une fois au demarrage.
    // ─────────────────────────────────────────────────────────────────────
    pub log_repo: Arc<dyn LogRepository>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    pub broadcaster: Arc<EventBroadcaster>,
    pub job_client: JobClient,
    pub discord_api: Arc<dyn DiscordApi>,
    pub api_key: String,
    /// Serveur Discord unique servi par cette installation. Vide =
    /// verrou desactive (cf. `middleware::single_guild`).
    pub guild_id: String,
    /// Relais vers la plateforme jeux. Seul chemin d'acces aux jeux
    /// depuis le web : le navigateur ne joint jamais nexus-api.
    pub nexus_games: Arc<crate::adapters::outbound::nexus_games::NexusGamesClient>,
    /// Token optionnel protégeant `/metrics` (vide = ouvert). Voir config.
    pub metrics_token: String,
    pub discord_bot_token: String,
    pub redis_client: redis::Client,
    pub cache: Option<Arc<RedisCache>>,
    /// Phase 7 B — Liste des Discord user_ids superadmin (env SUPERADMIN_USER_IDS).
    /// Utilisee pour gater les endpoints globaux non scoped par guild (ex: /purge/logs).
    pub superadmin_user_ids: Arc<Vec<String>>,

    // ─────────────────────────────────────────────────────────────────────
    // NE PAS utiliser depuis les handlers — passer par un repository
    // outbound (ports/outbound/*). Ce champ n'existe que pour le bootstrap
    // (construction des repositories Pg*) et les tests d'integration
    // (tests/test_helpers.rs) qui construisent AppState hors du crate.
    // ─────────────────────────────────────────────────────────────────────
    #[doc = "Reserve au bootstrap et aux tests d'integration. Aucun handler \
             inbound ne doit executer de SQL via ce pool : creer/utiliser un \
             port outbound (ex: SystemProbe pour les sondes sante)."]
    pub pg_pool: sqlx::PgPool,
}

// `bot_config_reminder_advance_secs` vivait ici ; il ne lisait que la config du
// bot `moderation-bot` et n'avait donc rien a faire sur l'etat global. Il est
// desormais une methode de `ModerationState`.
