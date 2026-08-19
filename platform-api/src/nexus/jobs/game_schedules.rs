//! Ouverture et fermeture automatiques des serveurs de jeu.
//!
//! Un serveur de soiree n'a pas besoin de tourner la journee. Ce passage lit
//! les plages horaires de chaque serveur et applique ce qu'elles disent :
//! demarrer, prevenir les joueurs, arreter.
//!
//! La regle vit dans le domaine (`game::schedule::decide`) ; ici on ne fait
//! qu'appliquer sa decision.

use platform_core::nexus::domain::entities::game::schedule::{
    decide, AutoSchedule, ScheduleAction, StopReason,
};
use platform_core::nexus::domain::entities::game::server::GameServerStatus;
use platform_core::nexus::domain::errors::DomainError;

use crate::nexus::bootstrap::AppState;

/// Auteur des actions automatiques dans le journal d'audit. Un nom explicite
/// plutot que celui du proprietaire : personne n'a clique.
const ACTEUR: &str = "horaires";

pub struct ScheduleReport {
    pub started: usize,
    pub stopped: usize,
    pub warned: usize,
    pub errors: usize,
}

pub async fn run(state: &AppState) -> Result<ScheduleReport, DomainError> {
    let horaires = state.game_schedule_repo.list_enabled().await?;
    let maintenant = chrono::Utc::now();

    let mut rapport = ScheduleReport {
        started: 0,
        stopped: 0,
        warned: 0,
        errors: 0,
    };

    for horaire in horaires {
        let Some(server) = state.game_server_repo.find_by_id(horaire.server_id).await? else {
            continue; // serveur supprime entre-temps
        };

        // Un serveur en pleine transition n'a pas a etre bouscule : le
        // demarrage prend des minutes, et l'arreter en route laisserait un
        // conteneur a moitie construit.
        if matches!(
            server.status,
            GameServerStatus::Starting | GameServerStatus::Stopping | GameServerStatus::Deleted
        ) {
            continue;
        }

        let running = server.status == GameServerStatus::Running;
        // L'annonce vaut pour la plage en cours. On la considere faite si elle
        // date de moins d'une heure : au-dela, c'est celle d'une plage passee.
        let deja_prevenu = horaire
            .last_warned_at
            .is_some_and(|t| (maintenant - t).num_minutes() < 60);

        let schedule = AutoSchedule {
            enabled: horaire.enabled,
            timezone: horaire.timezone.clone(),
            ranges: horaire.ranges.clone(),
            warn_minutes: horaire.warn_minutes,
            closes_at: server.closes_at,
        };

        match decide(&schedule, running, deja_prevenu, maintenant) {
            ScheduleAction::Nothing => {}

            ScheduleAction::Start => {
                match state.game_servers_uc.start(server.id, ACTEUR).await {
                    Ok(()) => {
                        rapport.started += 1;
                        // Nouvelle plage : le preavis de la precedente ne doit
                        // pas empecher celui de ce soir.
                        let _ = state.game_schedule_repo.clear_warning(server.id).await;
                        tracing::info!(server_id = %server.id, nom = %server.name, "horaires : serveur ouvert");
                    }
                    Err(error) => {
                        tracing::warn!(%error, server_id = %server.id, "horaires : ouverture impossible");
                        rapport.errors += 1;
                    }
                }
            }

            ScheduleAction::Warn { minutes_left } => {
                // L'annonce passe par le jeu lui-meme : c'est le seul endroit
                // ou un joueur en pleine partie la lira.
                let message = format!(
                    "Le serveur ferme dans {minutes_left} minutes. Pensez a vous mettre a l'abri."
                );
                let commande = format!("Broadcast {}", message.replace(' ', "_"));
                match state
                    .game_servers_uc
                    .execute_rcon(server.id, &commande, ACTEUR)
                    .await
                {
                    Ok(_) => {
                        rapport.warned += 1;
                        if let Err(error) = state.game_schedule_repo.mark_warned(server.id).await {
                            tracing::warn!(%error, server_id = %server.id, "horaires : preavis non enregistre");
                        }
                    }
                    Err(error) => {
                        // Un jeu sans RCON ne peut pas prevenir : on note le
                        // preavis quand meme, sinon on reessaierait chaque
                        // minute jusqu'a la fermeture.
                        tracing::info!(%error, server_id = %server.id, "horaires : preavis non transmis au jeu");
                        let _ = state.game_schedule_repo.mark_warned(server.id).await;
                    }
                }
            }

            ScheduleAction::Stop { reason } => {
                match state.game_servers_uc.stop(server.id, ACTEUR).await {
                    Ok(()) => {
                        rapport.stopped += 1;
                        let _ = state.game_schedule_repo.clear_warning(server.id).await;
                        let motif = match reason {
                            StopReason::OutsideRange => "fin de plage horaire",
                            StopReason::SessionOver => "fin de session",
                        };
                        tracing::info!(server_id = %server.id, nom = %server.name, motif, "horaires : serveur ferme");
                    }
                    Err(error) => {
                        tracing::warn!(%error, server_id = %server.id, "horaires : fermeture impossible");
                        rapport.errors += 1;
                    }
                }
            }
        }
    }

    Ok(rapport)
}
