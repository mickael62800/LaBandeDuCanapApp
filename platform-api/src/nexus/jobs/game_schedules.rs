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

use chrono::Utc;
use platform_core::nexus::application::game::config_loader::load_game_portal_config;
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
            opens_at: server.ip_reveal_at,
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
                            StopReason::NotYetOpen => "session pas encore ouverte",
                            StopReason::OutsideRange => "fin de plage horaire",
                            StopReason::SessionOver => "fin de session",
                        };
                        tracing::info!(server_id = %server.id, nom = %server.name, motif, "horaires : serveur ferme");

                        // ARCHIVE DU MONDE : UNE FERMETURE DE PLAGE EST LA MEME
                        // FENETRE QU'UN REDEMARRAGE.
                        //
                        // Le jeu vient de sauvegarder, le conteneur est arrete,
                        // rien ne le relancera avant demain : le monde est
                        // complet sur le disque, sans ecriture en cours. C'est
                        // exactement la condition qui rend l'archive fiable, et
                        // seul le redemarrage programme en profitait.
                        //
                        // Un serveur de soiree, qui ne tourne QUE sur des
                        // plages, n'etait donc jamais archive par
                        // l'application. Il ne restait que la sauvegarde
                        // nocturne du systeme — utile, mais exterieure a
                        // l'application et prise a une heure qui n'a aucun
                        // rapport avec la fin de la partie.
                        //
                        // La fin de SESSION la merite plus encore : c'est le
                        // dernier etat du monde, et il ne sera plus jamais
                        // recalcule.
                        //
                        // Les garde-fous de `archiver_le_monde` s'appliquent
                        // tels quels : interrupteur de configuration, volume
                        // existant, et delai minimal entre deux archives. Une
                        // plage qui se ferme chaque soir ne produit donc pas
                        // plus d'une archive par jour.
                        archiver_le_monde(state, &server).await;
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

    // 4. Archive du monde, conteneur arrete.
    //
    // C'est ICI, et nulle part ailleurs, que le monde est complet sur le disque
    // sans ecriture en cours : le jeu vient de sauvegarder, le conteneur ne
    // tourne plus, la relance n'a pas eu lieu. Aucune tache periodique ne peut
    // reproduire cette fenetre — une copie prise a chaud peut contenir un
    // fichier a moitie ecrit, ce qui ne se decouvre qu'au moment de restaurer.
    //
    // L'appel est ATTENDU, pas differe : demarrer pendant que tar lit donnerait
    // exactement l'incoherence qu'on cherche a eviter. Le cout est mesure —
    // une vingtaine de secondes pour 5 Go — et c'est du temps d'indisponibilite
    // assume, une fois par jour au plus.
    archiver_le_monde(state, server).await;

    // 5. Relance.
    state.game_servers_uc.start(server.id, ACTEUR).await
}

/// Nom de l'archive d'un monde : nom du serveur assaini, puis horodatage.
///
/// Le nom d'un serveur peut contenir des espaces (`chk_game_servers_name`), et
/// l'agent REFUSE tout nom de fichier contenant un separateur ou une remontee.
/// Sans assainissement, un serveur nomme « ../x » ferait echouer chaque archive
/// en silence, et un nom a espaces donnerait des fichiers penibles a manipuler.
///
/// L'horodatage n'est pas decoratif : c'est lui qui rend les archives
/// distinctes, donc conservables cote a cote sur la duree de retention.
fn nom_archive(nom_serveur: &str, maintenant: chrono::DateTime<Utc>) -> String {
    let assaini: String = nom_serveur
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{}-{}.tar", assaini, maintenant.format("%Y%m%d-%H%M%S"))
}

/// Archive le monde si la configuration le demande et que le delai est ecoule.
///
/// Ne remonte JAMAIS d'erreur : un serveur qui resterait eteint parce que sa
/// sauvegarde a echoue serait un remede pire que le mal. Tout echec est
/// journalise et la relance suit son cours.
async fn archiver_le_monde(state: &AppState, server: &GameServer) {
    let config = match load_game_portal_config(&state.bot_config_repo, &server.guild_id).await {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!(%error, server_id = %server.id, "archive : configuration illisible");
            return;
        }
    };
    if !config.backup_on_restart {
        return;
    }

    // Un serveur jamais demarre n'a pas de volume, donc pas de monde.
    let Some(volume) = server.volume_name.as_deref() else {
        return;
    };

    match state.game_backup_repo.last_auto_backup_at(server.id).await {
        Ok(Some(derniere)) => {
            let ecoulees = (Utc::now() - derniere).num_hours();
            if ecoulees < config.backup_min_interval_hours {
                tracing::debug!(
                    server_id = %server.id,
                    ecoulees,
                    minimum = config.backup_min_interval_hours,
                    "archive : delai non ecoule, on passe"
                );
                return;
            }
        }
        Ok(None) => {}
        Err(error) => {
            // Si la base ne repond pas, l'enregistrement echouerait aussi :
            // on produirait une archive que rien ne referencerait, et qu'aucune
            // purge ne supprimerait jamais.
            tracing::warn!(%error, server_id = %server.id, "archive : dernier passage illisible");
            return;
        }
    }

    let nom_fichier = nom_archive(&server.name, Utc::now());

    let debut = std::time::Instant::now();
    match state
        .game_container_runtime
        .archive_volume(volume, &nom_fichier)
        .await
    {
        Ok(archive) => {
            tracing::info!(
                server_id = %server.id,
                nom = %server.name,
                chemin = %archive.path,
                taille = archive.size_bytes,
                duree_ms = debut.elapsed().as_millis(),
                "archive : monde sauvegarde a froid"
            );
            if let Err(error) = state
                .game_backup_repo
                .record(server.id, &archive.path, archive.size_bytes as i64, "auto")
                .await
            {
                // L'archive existe mais rien ne la designe : la purge ne la
                // supprimera pas, et l'interface ne la listera pas.
                tracing::warn!(%error, server_id = %server.id, chemin = %archive.path, "archive : ecrite mais non enregistree");
            }
        }
        Err(error) => {
            tracing::warn!(%error, server_id = %server.id, "archive : sauvegarde du monde impossible");
        }
    }
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

#[cfg(test)]
mod tests_archive {
    use super::nom_archive;
    use chrono::{TimeZone, Utc};

    fn instant() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 23, 3, 0, 0).unwrap()
    }

    #[test]
    fn le_nom_porte_le_serveur_et_l_horodatage() {
        assert_eq!(
            nom_archive("palworld", instant()),
            "palworld-20260823-030000.tar"
        );
    }

    #[test]
    fn les_espaces_deviennent_des_soulignes() {
        // `chk_game_servers_name` autorise les espaces : « Le Canap sur
        // Palworld » est un nom valide en base.
        assert_eq!(
            nom_archive("Le Canap sur Palworld", instant()),
            "Le_Canap_sur_Palworld-20260823-030000.tar"
        );
    }

    #[test]
    fn un_nom_qui_ressemble_a_un_chemin_est_neutralise() {
        // L'agent refuse tout nom contenant un separateur ou une remontee :
        // sans cet assainissement, un tel serveur verrait CHACUNE de ses
        // archives echouer, et le journal seul le dirait.
        let obtenu = nom_archive("../../etc/passwd", instant());
        assert!(!obtenu.contains('/'), "{obtenu}");
        assert!(!obtenu.contains(".."), "{obtenu}");
        assert!(!obtenu.starts_with('.'), "{obtenu}");
    }

    #[test]
    fn deux_archives_du_meme_serveur_ne_se_recouvrent_pas() {
        // A la seconde pres : deux redemarrages du meme jour doivent donner
        // deux fichiers, pas un seul ecrase par l'autre.
        let a = nom_archive("x", Utc.with_ymd_and_hms(2026, 8, 23, 3, 0, 0).unwrap());
        let b = nom_archive("x", Utc.with_ymd_and_hms(2026, 8, 23, 3, 0, 1).unwrap());
        assert_ne!(a, b);
    }
}
