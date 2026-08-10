//! Workers de fond lances apres construction de l'AppState.

use crate::adapters::inbound::http::state::AppState;

/// A appeler apres construction de AppState pour lancer les workers de fond.
pub fn spawn_security_workers(state: &AppState) {
    crate::adapters::outbound::system::alerts_dispatcher::spawn(
        state.pg_pool.clone(),
        state.redis_client.clone(),
        state.ops.container_monitor.clone(),
    );
    // Planificateur de sauvegardes automatiques (config guild-backup-bot).
    crate::bootstrap::backup_scheduler::spawn(state.clone());
}
