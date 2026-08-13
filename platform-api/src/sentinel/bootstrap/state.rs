//! `AppState` — la composition root de l'API.
//!
//! # Decoupage par domaine
//!
//! `AppState` etait un objet plat de ~100 champs. C'est desormais un assemblage
//! de sous-etats par domaine et d'un `SharedState` transversal. Chaque
//! sous-etat implemente `FromRef<AppState>`,
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
//! Les handlers transversaux declarent une vue de capacites etroite depuis
//! `capabilities`; aucun handler HTTP n'extrait plus directement `AppState`.

pub mod ai;
pub mod audit;
pub mod capabilities;
pub mod community;
pub mod guild_backup;
pub mod jobs;
pub mod moderation;
pub mod ops;
pub mod shared;
pub mod system;

pub use ai::AiState;
pub use audit::AuditState;
pub use capabilities::{
    BotPersistenceState, CacheStatsState, DashboardState, NexusGamesState, PurgeState,
    VoiceChannelsState,
};
pub use community::CommunityState;
pub use guild_backup::GuildBackupState;
pub use jobs::InternalJobsState;
pub use moderation::ModerationState;
pub use ops::OpsState;
pub use shared::SharedState;
pub use system::SystemState;

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
    pub jobs: InternalJobsState,
    pub shared: SharedState,
}

// `bot_config_reminder_advance_secs` vivait ici ; il ne lisait que la config du
// bot `moderation-bot` et n'avait donc rien a faire sur l'etat global. Il est
// desormais une methode de `ModerationState`.
