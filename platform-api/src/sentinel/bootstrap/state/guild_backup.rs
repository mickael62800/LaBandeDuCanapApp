//! Etat du domaine guild_backup : snapshots de serveur et re-attribution de roles.

use std::sync::Arc;

use axum::extract::FromRef;
use platform_core::sentinel::ports::inbound::guild_backup::manage_pending_role_grants::ManagePendingRoleGrantsUseCase;
use platform_core::sentinel::ports::inbound::guild_backup::manage_snapshots::ManageGuildSnapshotsUseCase;
use platform_core::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

use crate::sentinel::adapters::outbound::ws::broadcaster::EventBroadcaster;
use crate::sentinel::bootstrap::state::AppState;

/// Ports de la sauvegarde / restauration de serveur.
///
/// Domaine volontairement etroit : la restauration est l'operation la plus
/// destructrice de l'API. Moins ces handlers ont acces a autre chose, mieux
/// c'est.
///
/// `broadcaster` et `bot_config_repo` sont transverses mais bien des
/// dependances de ce domaine : le restore diffuse sa progression sur la WS et
/// lit les reglages du serveur. Les declarer ici les rend visibles — dans
/// l'ancien `AppState` plat, elles etaient indiscernables des 98 autres champs.
#[derive(Clone)]
pub struct GuildBackupState {
    pub guild_snapshots_uc: Arc<dyn ManageGuildSnapshotsUseCase>,
    pub pending_role_grants_uc: Arc<dyn ManagePendingRoleGrantsUseCase>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    pub broadcaster: Arc<EventBroadcaster>,
    /// Secret partage bot <-> API, pour signer `guild_backup:capture_requested`
    /// et `guild_backup:restore_requested`. Le bus Redis est commun a toutes les
    /// plateformes : sans signature, y publier suffisait a faire vider un
    /// serveur Discord. Meme role que dans `SystemState` pour `guild_reset`.
    pub api_key: String,
}

impl FromRef<AppState> for GuildBackupState {
    fn from_ref(state: &AppState) -> Self {
        state.guild_backup.clone()
    }
}
