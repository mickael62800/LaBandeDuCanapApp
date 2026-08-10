//! Workers de fond lances apres construction de l'AppState.

use crate::adapters::inbound::http::state::AppState;

/// A appeler apres construction de AppState pour lancer les workers de fond.
pub fn spawn_security_workers(state: &AppState) {
    // Le dispatcher d'alertes et la surveillance des conteneurs vivent
    // desormais dans `ops-api` : ils evaluent la MACHINE, pas Discord.
    // Planificateur de sauvegardes automatiques (config guild-backup-bot).
    crate::bootstrap::backup_scheduler::spawn(state.clone());
}
