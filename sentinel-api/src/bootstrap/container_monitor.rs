//! Poll Docker en arriere-plan, detecte added/removed/state_changed/
//! image_changed et logue dans `server_events`. Garde un snapshot + les 200
//! derniers changements.
//!
//! # Deux dettes corrigees ici
//!
//! Ce job ouvrait sa PROPRE connexion bollard (`Docker::connect_with_local_defaults`)
//! et ecrivait en SQL brut, court-circuitant deux fois l'hexagone. Il passe
//! desormais par les ports `DockerHost` et `ServerEventRepository` : il ne sait
//! plus ni que Docker est derriere un socket ou un agent HTTP, ni qu'il ecrit
//! dans Postgres. C'est ce qui a permis de retirer le socket de ce processus.
//!
//! Il reste heberge dans le bootstrap de l'API faute d'`exploitation-worker` :
//! c'est sa place naturelle, un job periodique n'ayant rien a faire dans un
//! processus HTTP.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use sentinel_core::ports::outbound::ops::docker_host::DockerHost;
use sentinel_core::ports::outbound::ops::server_event_repository::ServerEventRepository;
use tokio::sync::RwLock;

use crate::adapters::inbound::http::handlers::system::security::{
    ContainerChangeEntry, ContainerSnapshot,
};

#[derive(Default, Debug, Clone)]
pub struct ContainerMonitorState {
    pub last_check: String,
    pub current: Vec<ContainerSnapshot>,
    pub recent_changes: Vec<ContainerChangeEntry>,
}

pub fn spawn(
    docker: Arc<dyn DockerHost>,
    server_events: Arc<dyn ServerEventRepository>,
) -> Arc<RwLock<ContainerMonitorState>> {
    let state = Arc::new(RwLock::new(ContainerMonitorState::default()));
    let st = state.clone();
    tokio::spawn(async move {
        let mut prev: HashMap<String, ContainerSnapshot> = HashMap::new();
        let mut first_run = true;
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            // `all = true` : un conteneur arrete reste un conteneur a surveiller,
            // c'est meme le cas qui interesse le plus (crash, arret imprevu).
            let conts = match docker.list_containers(true).await {
                Ok(c) => c,
                Err(e) => {
                    // L'agent Docker peut etre absent (profil optionnel) ou en
                    // cours de redemarrage : on retente au tour suivant plutot
                    // que de tuer la boucle, sinon un redemarrage de l'agent
                    // arreterait la surveillance jusqu'au prochain deploiement.
                    tracing::warn!("container_monitor list : {e}");
                    continue;
                }
            };
            let now = chrono::Utc::now().to_rfc3339();
            let mut current_map: HashMap<String, ContainerSnapshot> = HashMap::new();
            let mut current_vec: Vec<ContainerSnapshot> = Vec::new();
            for c in conts {
                if c.id.is_empty() {
                    continue;
                }
                let snap = ContainerSnapshot {
                    id: c.id.clone(),
                    // Docker renvoie une liste de noms prefixes d'un `/` ; le
                    // premier est le nom usuel du conteneur.
                    name: c
                        .names
                        .first()
                        .map(|s| s.trim_start_matches('/').to_string())
                        .unwrap_or_default(),
                    image: c.image.clone(),
                    state: c.state.clone(),
                    started_at: Some(c.created.to_string()),
                };
                current_map.insert(c.id.clone(), snap.clone());
                current_vec.push(snap);
            }

            // Diff (skip first run pour pas tout marquer comme nouveau)
            let mut changes: Vec<ContainerChangeEntry> = Vec::new();
            if !first_run {
                for (id, snap) in &current_map {
                    match prev.get(id) {
                        None => changes.push(ContainerChangeEntry {
                            timestamp: now.clone(),
                            kind: "added".into(),
                            container: snap.clone(),
                            previous: None,
                        }),
                        Some(p) if p.image != snap.image => changes.push(ContainerChangeEntry {
                            timestamp: now.clone(),
                            kind: "image_changed".into(),
                            container: snap.clone(),
                            previous: Some(p.clone()),
                        }),
                        Some(p) if p.state != snap.state => changes.push(ContainerChangeEntry {
                            timestamp: now.clone(),
                            kind: "state_changed".into(),
                            container: snap.clone(),
                            previous: Some(p.clone()),
                        }),
                        _ => {}
                    }
                }
                for (id, snap) in &prev {
                    if !current_map.contains_key(id) {
                        changes.push(ContainerChangeEntry {
                            timestamp: now.clone(),
                            kind: "removed".into(),
                            container: snap.clone(),
                            previous: None,
                        });
                    }
                }
            }
            first_run = false;
            prev = current_map;

            // Logue chaque change dans server_events
            for ch in &changes {
                let action = format!("docker.{}", ch.kind);
                let target = format!(
                    "{} ({})",
                    ch.container.name,
                    &ch.container.id[..12.min(ch.container.id.len())]
                );
                let details = serde_json::to_value(ch).unwrap_or(serde_json::Value::Null);
                let severity = if ch.kind == "removed" || ch.kind == "added" {
                    "warn"
                } else {
                    "info"
                };
                let _ = server_events
                    .record(
                        "system:container_monitor",
                        None,
                        &action,
                        Some(&target),
                        severity,
                        details,
                    )
                    .await;
            }

            // Update state public + garde 24h max
            let mut w = st.write().await;
            w.last_check = now.clone();
            w.current = current_vec;
            for ch in changes {
                w.recent_changes.push(ch);
            }
            // Trim : garde 200 derniers changes
            if w.recent_changes.len() > 200 {
                let drop_n = w.recent_changes.len() - 200;
                w.recent_changes.drain(0..drop_n);
            }
        }
    });
    state
}
