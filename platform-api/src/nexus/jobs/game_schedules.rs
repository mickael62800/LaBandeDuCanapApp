//! Pilotage des serveurs de jeu dans le temps.
//!
//! Deux systemes exclusifs, choisis par serveur (`mode`) :
//!
//!   - **plages d'ouverture** : un serveur de soiree n'a pas besoin de tourner
//!     la journee. On l'ouvre, on previent, on le ferme ;
//!   - **permanence** : le serveur tourne en continu, et redemarre a intervalle
//!     regulier. Un jeu qui tourne des jours d'affilee ne rend pas la memoire
//!     qu'il prend et finit par ramer, puis par tomber.
//!
//! La regle vit dans le domaine (`game::schedule::decide`) ; ici on ne fait
//! qu'appliquer sa decision.
//!
//! ## L'annonce part toujours en double
//!
//! Le message RCON touche ceux qui JOUENT — c'est le seul endroit ou un joueur
//! en pleine partie le lira. L'annonce Discord touche ceux qui s'appretent a se
//! connecter. Aucune des deux ne remplace l'autre.

use platform_core::nexus::domain::entities::game::schedule::{
    decide, next_restart_at, AutoSchedule, ScheduleAction, ScheduleMode, StopReason,
};
use platform_core::nexus::domain::entities::game::server::{GameServer, GameServerStatus};
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::events::game_events::{
    SERVER_RESTARTED, SERVER_RESTART_WARNING,
};

use crate::nexus::bootstrap::AppState;

/// Auteur des actions automatiques dans le journal d'audit. Un nom explicite
/// plutot que celui du proprietaire : personne n'a clique.
const ACTEUR: &str = "horaires";

/// Cle de la commande d'annonce dans le catalogue du modele de jeu.
///
/// Chaque jeu parle sa propre langue (`Broadcast`, `say`, `BroadcastMessage`) :
/// le gabarit vit en base, par modele. Composer la commande a la main ici
/// n'aurait marche que pour le jeu qui a servi a l'ecrire.
const CMD_ANNONCE: &str = "broadcast";

/// Cle de la commande de sauvegarde du monde.
const CMD_SAUVEGARDE: &str = "save";

pub struct ScheduleReport {
    pub started: usize,
    pub stopped: usize,
    pub warned: usize,
    pub restarted: usize,
    pub errors: usize,
}

pub async fn run(state: &AppState) -> Result<ScheduleReport, DomainError> {
    let horaires = state.game_schedule_repo.list_enabled().await?;
    let maintenant = chrono::Utc::now();

    let mut rapport = ScheduleReport {
        started: 0,
        stopped: 0,
        warned: 0,
        restarted: 0,
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
        //
        // Ce raccourci ne vaut QUE pour les plages : en permanence, un
        // intervalle d'une heure le rendrait faux, et le domaine y compare donc
        // les marqueurs au creneau lui-meme.
        let deja_prevenu = horaire
            .last_warned_at
            .is_some_and(|t| (maintenant - t).num_minutes() < 60);

        let schedule = AutoSchedule {
            enabled: horaire.enabled,
            mode: horaire.mode,
            timezone: horaire.timezone.clone(),
            ranges: horaire.ranges.clone(),
            warn_minutes: horaire.warn_minutes,
            closes_at: server.closes_at,
            restart_interval_hours: horaire.restart_interval_hours,
            restart_anchor_minute: horaire.restart_anchor_minute,
            last_restart_at: horaire.last_restart_at,
            last_warned_at: horaire.last_warned_at,
            last_final_warned_at: horaire.last_final_warned_at,
        };

        match decide(&schedule, running, deja_prevenu, maintenant) {
            ScheduleAction::Nothing => {}

            ScheduleAction::Start => match state.game_servers_uc.start(server.id, ACTEUR).await {
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
            },

            ScheduleAction::Warn { minutes_left } => {
                let message = format!(
                    "Le serveur ferme dans {minutes_left} minutes. Pensez a vous mettre a l'abri."
                );
                annoncer_dans_le_jeu(state, &server, &message).await;
                // Le preavis est note meme si le jeu n'a pas pu le relayer :
                // sinon on reessaierait chaque minute jusqu'a la fermeture.
                if let Err(error) = state.game_schedule_repo.mark_warned(server.id).await {
                    tracing::warn!(%error, server_id = %server.id, "horaires : preavis non enregistre");
                }
                rapport.warned += 1;
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

            ScheduleAction::RestartWarn { minutes_left } => {
                let message = format!(
                    "Redemarrage du serveur dans {minutes_left} minutes. \
                     Mettez-vous a l'abri et sauvegardez votre progression."
                );
                annoncer_dans_le_jeu(state, &server, &message).await;
                annoncer_sur_discord(state, &server, &schedule, minutes_left, maintenant).await;
                if let Err(error) = state.game_schedule_repo.mark_warned(server.id).await {
                    tracing::warn!(%error, server_id = %server.id, "permanence : preavis non enregistre");
                }
                rapport.warned += 1;
            }

            ScheduleAction::RestartFinalWarn => {
                // Une minute : le temps de poser ce qu'on porte et de sortir.
                // C'est la seule annonce qui sert vraiment a se deconnecter.
                let message = "Redemarrage dans 1 minute. Deconnectez-vous maintenant.";
                annoncer_dans_le_jeu(state, &server, message).await;
                if let Err(error) = state.game_schedule_repo.mark_final_warned(server.id).await {
                    tracing::warn!(%error, server_id = %server.id, "permanence : annonce finale non enregistree");
                }
                rapport.warned += 1;
            }

            ScheduleAction::Restart => match redemarrer(state, &server).await {
                Ok(()) => {
                    rapport.restarted += 1;
                    // Le marqueur tombe APRES coup : si le redemarrage echoue,
                    // le creneau reste ouvert et sera retente au passage
                    // suivant, dans la limite de la tolerance de retard.
                    if let Err(error) = state.game_schedule_repo.mark_restarted(server.id).await {
                        tracing::warn!(%error, server_id = %server.id, "permanence : redemarrage non enregistre");
                    }
                    state
                        .events
                        .publish(
                            SERVER_RESTARTED,
                            serde_json::json!({
                                "server_id": server.id.to_string(),
                                "guild_id": server.guild_id,
                                "name": server.name,
                            }),
                        )
                        .await;
                    tracing::info!(server_id = %server.id, nom = %server.name, "permanence : serveur redemarre");
                }
                Err(error) => {
                    tracing::warn!(%error, server_id = %server.id, "permanence : redemarrage impossible");
                    rapport.errors += 1;
                }
            },
        }
    }

    Ok(rapport)
}

/// Redemarrage propre : on sauvegarde, on previent, on arrete, on relance.
///
/// L'ordre n'est pas negociable. Sauvegarder APRES avoir arrete ne sauve rien,
/// et arreter sans sauvegarder perd tout ce qui n'avait pas encore ete ecrit —
/// sur certains jeux, plusieurs minutes de construction.
async fn redemarrer(state: &AppState, server: &GameServer) -> Result<(), DomainError> {
    // 1. Ecrire le monde sur le disque. Un jeu sans commande de sauvegarde le
    //    fait de lui-meme a l'arret ; on ne bloque pas le redemarrage pour ca.
    match state
        .game_servers_uc
        .run_catalog_command(server.id, CMD_SAUVEGARDE, &[], ACTEUR)
        .await
    {
        Ok(_) => {
            tracing::info!(server_id = %server.id, "permanence : monde sauvegarde avant redemarrage")
        }
        Err(error) => {
            tracing::info!(%error, server_id = %server.id, "permanence : pas de sauvegarde explicite pour ce jeu")
        }
    }

    // 2. Dernier mot aux joueurs encore connectes.
    annoncer_dans_le_jeu(state, server, "Redemarrage du serveur en cours...").await;

    // 3. Arret gracieux : le use case passe par `docker stop -t`, qui laisse au
    //    jeu le delai de grace configure avant de le tuer.
    state.game_servers_uc.stop(server.id, ACTEUR).await?;

    // 4. Relance.
    state.game_servers_uc.start(server.id, ACTEUR).await
}

/// Annonce dans le jeu, via la commande du catalogue propre au modele.
///
/// Ne remonte jamais d'erreur : un jeu sans RCON, ou sans commande d'annonce,
/// ne doit pas empecher la fermeture ou le redemarrage d'avoir lieu.
async fn annoncer_dans_le_jeu(state: &AppState, server: &GameServer, message: &str) {
    let params = [("message".to_string(), message.to_string())];
    match state
        .game_servers_uc
        .run_catalog_command(server.id, CMD_ANNONCE, &params, ACTEUR)
        .await
    {
        Ok(_) => tracing::info!(server_id = %server.id, "annonce transmise au jeu"),
        Err(error) => {
            tracing::info!(%error, server_id = %server.id, "annonce non transmise au jeu")
        }
    }
}

/// Annonce sur Discord. Porte l'heure du redemarrage pour que le bot puisse
/// l'afficher sans refaire le calcul de fuseau.
async fn annoncer_sur_discord(
    state: &AppState,
    server: &GameServer,
    schedule: &AutoSchedule,
    minutes_left: u16,
    maintenant: chrono::DateTime<chrono::Utc>,
) {
    let prochain = next_restart_at(schedule, maintenant).map(|t| t.to_rfc3339());
    state
        .events
        .publish(
            SERVER_RESTART_WARNING,
            serde_json::json!({
                "server_id": server.id.to_string(),
                "guild_id": server.guild_id,
                "name": server.name,
                "minutes_left": minutes_left,
                "restart_at": prochain,
            }),
        )
        .await;
}

/// Le mode est-il celui d'une permanence ? Utilitaire pour les appelants qui
/// affichent le prochain redemarrage sans rejouer la decision.
pub fn est_permanence(mode: ScheduleMode) -> bool {
    mode == ScheduleMode::Restart
}
