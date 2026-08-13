//! Surveillance periodique des conteneurs de l'hote par `ops-agent`.
//!
//! Interroge Docker chaque minute, compare avec le relevé precedent et
//! journalise chaque changement dans `server_events`. L'instantane courant et
//! les derniers changements sont PUBLIES dans Redis (`REDIS_STATE_KEY`) ;
//! `ops-api` les sert sur `/containers/changes` en LISANT cette cle — les deux
//! processus ne partagent pas de memoire.
//!
//! Le worker compare deux instantanés, journalise les changements et publie
//! l'état dans Redis pour qu'`ops-api` puisse le servir sans partager sa
//! mémoire avec le worker.
//!
//! # Ce qui a ete corrige en chemin
//!
//! Ce job vivait dans `sentinel-api`, ouvrait sa PROPRE connexion bollard et
//! ecrivait en SQL brut : il court-circuitait deux fois l'hexagone. Il passe
//! desormais par les ports `DockerHost` et `ServerEventRepository`, et la
//! comparaison de deux relevés — la seule regle metier ici — vit dans
//! `platform_core::ops::domain::entities::container_monitor::detect_changes`, ou elle est
//! testee sans Docker ni base.
//!
//! # Pourquoi un worker et non l'API
//!
//! La surveillance est un travail de fond periodique, sans requete entrante : sa
//! place est dans le worker, pas dans l'API. L'etat ephemere transite par Redis
//! (une cle a TTL), ce qui decouple le producteur (worker) du consommateur
//! (API) sans base ni memoire partagee.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use platform_core::ops::domain::entities::container_monitor::{
    detect_changes, ContainerMonitorState, ContainerSnapshot, REDIS_STATE_KEY,
};
use platform_core::ops::domain::entities::server_event::NewServerEvent;
use platform_core::ops::ports::outbound::docker_host::DockerHost;
use platform_core::ops::ports::outbound::server_event_repository::ServerEventRepository;
use tokio::sync::RwLock;

/// Intervalle entre deux relevés. Une minute suffit : on cherche a expliquer
/// un incident apres coup, pas a reagir en temps reel.
const POLL_INTERVAL: Duration = Duration::from_secs(60);

pub type SharedMonitorState = Arc<RwLock<ContainerMonitorState>>;

pub fn spawn(
    docker: Arc<dyn DockerHost>,
    server_events: Arc<dyn ServerEventRepository>,
    redis_client: redis::Client,
) -> SharedMonitorState {
    let state: SharedMonitorState = Arc::new(RwLock::new(ContainerMonitorState::default()));
    let shared = state.clone();

    tokio::spawn(async move {
        let mut previous: HashMap<String, ContainerSnapshot> = HashMap::new();
        // Le premier relevé sert de reference : sans lui, tous les conteneurs
        // deja en place seraient rapportes comme « ajoutes » au demarrage.
        let mut first_run = true;

        // `interval` : le premier `tick()` est immediat, donc la reference se
        // construit des le demarrage (et non apres une minute), sans produire
        // d'evenements `Added`. `Skip` evite d'accumuler les ticks si un relevé
        // Docker deborde l'intervalle.
        let mut ticker = tokio::time::interval(POLL_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;

            // `all = true` : un conteneur arrete reste a surveiller — c'est
            // meme le cas qui interesse le plus (crash, arret imprevu).
            let containers = match docker.list_containers(true).await {
                Ok(containers) => containers,
                Err(error) => {
                    // L'agent Docker peut etre absent ou en cours de
                    // redemarrage : on retente au tour suivant plutot que de
                    // tuer la boucle, sinon un simple redemarrage de l'agent
                    // arreterait la surveillance jusqu'au prochain deploiement.
                    tracing::warn!(%error, "relevé des conteneurs impossible");
                    continue;
                }
            };

            let now = chrono::Utc::now().to_rfc3339();
            let mut current: HashMap<String, ContainerSnapshot> = HashMap::new();
            for c in containers {
                if c.id.is_empty() {
                    continue;
                }
                current.insert(
                    c.id.clone(),
                    ContainerSnapshot {
                        id: c.id.clone(),
                        // Docker renvoie une liste de noms prefixes d'un `/` ;
                        // le premier est le nom usuel du conteneur.
                        name: c
                            .names
                            .first()
                            .map(|n| n.trim_start_matches('/').to_string())
                            .unwrap_or_default(),
                        image: c.image.clone(),
                        state: c.state.clone(),
                        started_at: Some(c.created.to_string()),
                    },
                );
            }

            let changes = if first_run {
                Vec::new()
            } else {
                detect_changes(&previous, &current, &now)
            };
            first_run = false;

            // Audit : tous les changements du relevé en UN insert groupe, au lieu
            // d'un aller-retour SQL par changement (couteux lors d'une recreation
            // massive de conteneurs).
            if !changes.is_empty() {
                let events: Vec<NewServerEvent> = changes
                    .iter()
                    .map(|change| {
                        let target = format!(
                            "{} ({})",
                            change.container.name,
                            &change.container.id[..12.min(change.container.id.len())]
                        );
                        NewServerEvent {
                            actor: "system:container_monitor".to_owned(),
                            actor_name: None,
                            action: change.kind.as_action().to_owned(),
                            target: Some(target),
                            severity: change.kind.severity().to_owned(),
                            details: serde_json::to_value(change)
                                .unwrap_or(serde_json::Value::Null),
                        }
                    })
                    .collect();
                if let Err(error) = server_events.record_batch(&events).await {
                    tracing::warn!(%error, "journalisation des changements impossible");
                }
            }

            let mut snapshot: Vec<ContainerSnapshot> = current.values().cloned().collect();
            // Ordre stable : sans tri, l'ordre d'un `HashMap` varie a chaque
            // relevé et la liste du back-office sautille sans raison.
            snapshot.sort_by(|a, b| a.name.cmp(&b.name));
            // Deplacement plutot que clone complet : `current` n'est plus relu
            // apres ce point, il devient directement la reference du prochain tour.
            previous = current;

            // Un seul verrou en ecriture : on applique puis on serialise l'etat
            // tant qu'on le tient, au lieu d'un write suivi d'un read.
            let encoded = {
                let mut state = shared.write().await;
                state.apply(now, snapshot, changes);
                serde_json::to_string(&*state)
            };
            match encoded {
                Ok(encoded) => match redis_client.get_multiplexed_async_connection().await {
                    Ok(mut connection) => {
                        if let Err(error) = redis::cmd("SET")
                            .arg(REDIS_STATE_KEY)
                            .arg(encoded)
                            .arg("EX")
                            .arg(300)
                            .query_async::<()>(&mut connection)
                            .await
                        {
                            tracing::warn!(%error, "publication du snapshot conteneurs impossible");
                        }
                    }
                    Err(error) => tracing::warn!(%error, "connexion Redis du monitor impossible"),
                },
                Err(error) => tracing::warn!(%error, "serialisation du snapshot impossible"),
            }
        }
    });

    state
}
