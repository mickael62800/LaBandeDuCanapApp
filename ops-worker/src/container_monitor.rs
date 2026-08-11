//! Surveillance periodique des conteneurs de l'hote par `ops-worker`.
//!
//! Interroge Docker chaque minute, compare avec le relevé precedent et
//! journalise chaque changement dans `server_events`. Garde en memoire
//! l'instantane courant et les derniers changements, servis par
//! `/containers/changes`.
//!
//! # Ce qui a ete corrige en chemin
//!
//! Ce job vivait dans `sentinel-api`, ouvrait sa PROPRE connexion bollard et
//! ecrivait en SQL brut : il court-circuitait deux fois l'hexagone. Il passe
//! desormais par les ports `DockerHost` et `ServerEventRepository`, et la
//! comparaison de deux relevés — la seule regle metier ici — vit dans
//! `ops_core::domain::entities::container_monitor::detect_changes`, ou elle est
//! testee sans Docker ni base.
//!
//! # Pourquoi dans l'API et non dans un worker
//!
//! L'instantane est partage EN MEMOIRE avec le handler qui le sert. Un worker
//! separe imposerait de le faire transiter par Redis ou Postgres, pour un etat
//! ephemere que personne ne relit apres coup. Le processus qui produit la
//! donnee est celui qui la sert : c'est le montage le plus simple qui marche.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use ops_core::domain::entities::container_monitor::{
    detect_changes, ContainerMonitorState, ContainerSnapshot, REDIS_STATE_KEY,
};
use ops_core::ports::outbound::docker_host::DockerHost;
use ops_core::ports::outbound::server_event_repository::ServerEventRepository;
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

        loop {
            tokio::time::sleep(POLL_INTERVAL).await;

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
            previous = current.clone();

            for change in &changes {
                let target = format!(
                    "{} ({})",
                    change.container.name,
                    &change.container.id[..12.min(change.container.id.len())]
                );
                let details = serde_json::to_value(change).unwrap_or(serde_json::Value::Null);
                if let Err(error) = server_events
                    .record(
                        "system:container_monitor",
                        None,
                        change.kind.as_action(),
                        Some(&target),
                        change.kind.severity(),
                        details,
                    )
                    .await
                {
                    tracing::warn!(%error, "journalisation d'un changement impossible");
                }
            }

            let mut snapshot: Vec<ContainerSnapshot> = current.values().cloned().collect();
            // Ordre stable : sans tri, l'ordre d'un `HashMap` varie a chaque
            // relevé et la liste du back-office sautille sans raison.
            snapshot.sort_by(|a, b| a.name.cmp(&b.name));

            shared.write().await.apply(now, snapshot, changes);

            let encoded = {
                let state = shared.read().await;
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
