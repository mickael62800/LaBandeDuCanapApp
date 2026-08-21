//! Surveillance des conteneurs de l'hote : instantane et detection des
//! changements.
//!
//! # Pourquoi ces types sont ici
//!
//! `ContainerSnapshot` et `ContainerChangeEntry` etaient definis dans un
//! HANDLER HTTP (`handlers/system/security.rs`) et consommes par la boucle de
//! surveillance du bootstrap. Des entites du domaine vivaient donc dans un
//! adaptateur entrant, et la logique de comparaison etait enfermee dans une
//! boucle `tokio` — donc intestable sans Docker ni base.
//!
//! `detect_changes` est desormais une fonction pure : elle compare deux
//! instantanes et rend la liste des changements. C'est la seule regle metier de
//! ce module, et elle se teste en trois lignes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Cle Redis partagee entre le producteur (`ops-worker`) et le lecteur
/// (`ops-api`) du snapshot.
pub const REDIS_STATE_KEY: &str = "ops:container_monitor:state";

/// Etat d'un conteneur a un instant donne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSnapshot {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub started_at: Option<String>,
}

/// Nature d'un changement observe entre deux relevés.
///
/// Etait une `String` libre. Un enum interdit la faute de frappe silencieuse
/// (`"image_change"` au lieu de `"image_changed"` passait sans bruit et rendait
/// le filtre cote web inoperant), tout en conservant le meme JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerChangeKind {
    Added,
    Removed,
    ImageChanged,
    StateChanged,
}

impl ContainerChangeKind {
    /// Apparition et disparition sont plus alarmantes qu'un simple changement
    /// d'etat : elles signalent un deploiement ou une suppression.
    pub fn severity(self) -> &'static str {
        match self {
            Self::Added | Self::Removed => "warn",
            Self::ImageChanged | Self::StateChanged => "info",
        }
    }

    pub fn as_action(self) -> &'static str {
        match self {
            Self::Added => "docker.added",
            Self::Removed => "docker.removed",
            Self::ImageChanged => "docker.image_changed",
            Self::StateChanged => "docker.state_changed",
        }
    }
}

/// Un changement observe, avec l'etat precedent quand il existe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerChangeEntry {
    pub timestamp: String,
    pub kind: ContainerChangeKind,
    pub container: ContainerSnapshot,
    pub previous: Option<ContainerSnapshot>,
}

/// Instantane courant + historique borne des changements.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerMonitorState {
    pub last_check: String,
    pub current: Vec<ContainerSnapshot>,
    pub recent_changes: Vec<ContainerChangeEntry>,
}

/// Nombre de changements conserves. Au-dela, les plus anciens sont oublies :
/// cet historique sert au diagnostic immediat, pas a l'archivage — les
/// evenements durables sont ecrits dans `server_events`.
pub const MAX_RECENT_CHANGES: usize = 200;

impl ContainerMonitorState {
    /// Applique un relevé : remplace l'instantane et empile les changements.
    pub fn apply(
        &mut self,
        now: String,
        current: Vec<ContainerSnapshot>,
        changes: Vec<ContainerChangeEntry>,
    ) {
        self.last_check = now;
        self.current = current;
        self.recent_changes.extend(changes);
        if self.recent_changes.len() > MAX_RECENT_CHANGES {
            let drop_n = self.recent_changes.len() - MAX_RECENT_CHANGES;
            self.recent_changes.drain(0..drop_n);
        }
    }
}

/// Compare deux relevés et rend les changements observes.
///
/// Un conteneur peut changer d'image ET d'etat entre deux relevés ; on ne
/// rapporte alors que le changement d'image, le plus significatif — un
/// redemarrage accompagne toujours un changement d'image, et rapporter les deux
/// ferait du bruit sans informer.
pub fn detect_changes(
    previous: &HashMap<String, ContainerSnapshot>,
    current: &HashMap<String, ContainerSnapshot>,
    now: &str,
) -> Vec<ContainerChangeEntry> {
    let mut changes = Vec::new();

    for (id, snap) in current {
        match previous.get(id) {
            None => changes.push(ContainerChangeEntry {
                timestamp: now.to_owned(),
                kind: ContainerChangeKind::Added,
                container: snap.clone(),
                previous: None,
            }),
            Some(before) if before.image != snap.image => changes.push(ContainerChangeEntry {
                timestamp: now.to_owned(),
                kind: ContainerChangeKind::ImageChanged,
                container: snap.clone(),
                previous: Some(before.clone()),
            }),
            Some(before) if before.state != snap.state => changes.push(ContainerChangeEntry {
                timestamp: now.to_owned(),
                kind: ContainerChangeKind::StateChanged,
                container: snap.clone(),
                previous: Some(before.clone()),
            }),
            _ => {}
        }
    }

    for (id, snap) in previous {
        if !current.contains_key(id) {
            changes.push(ContainerChangeEntry {
                timestamp: now.to_owned(),
                kind: ContainerChangeKind::Removed,
                container: snap.clone(),
                previous: None,
            });
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: &str, image: &str, state: &str) -> ContainerSnapshot {
        ContainerSnapshot {
            id: id.into(),
            name: format!("nom-{id}"),
            image: image.into(),
            state: state.into(),
            started_at: None,
        }
    }

    fn map(items: &[ContainerSnapshot]) -> HashMap<String, ContainerSnapshot> {
        items.iter().map(|s| (s.id.clone(), s.clone())).collect()
    }

    #[test]
    fn aucun_changement_quand_rien_ne_bouge() {
        let a = map(&[snap("1", "img:1", "running")]);
        assert!(detect_changes(&a, &a, "t").is_empty());
    }

    #[test]
    fn detecte_apparition_et_disparition() {
        let avant = map(&[snap("1", "img:1", "running")]);
        let apres = map(&[snap("2", "img:2", "running")]);
        let changes = detect_changes(&avant, &apres, "t");
        assert_eq!(changes.len(), 2);
        assert!(changes
            .iter()
            .any(|c| c.kind == ContainerChangeKind::Added && c.container.id == "2"));
        assert!(changes
            .iter()
            .any(|c| c.kind == ContainerChangeKind::Removed && c.container.id == "1"));
    }

    #[test]
    fn changement_d_image_prime_sur_changement_d_etat() {
        // Un redemarrage accompagne toujours un changement d'image : rapporter
        // les deux ferait du bruit sans rien apprendre.
        let avant = map(&[snap("1", "img:1", "running")]);
        let apres = map(&[snap("1", "img:2", "restarting")]);
        let changes = detect_changes(&avant, &apres, "t");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, ContainerChangeKind::ImageChanged);
        assert_eq!(changes[0].previous.as_ref().unwrap().image, "img:1");
    }

    #[test]
    fn severite_plus_forte_pour_apparition_et_disparition() {
        assert_eq!(ContainerChangeKind::Added.severity(), "warn");
        assert_eq!(ContainerChangeKind::Removed.severity(), "warn");
        assert_eq!(ContainerChangeKind::StateChanged.severity(), "info");
    }

    #[test]
    fn historique_borne_a_200_entrees() {
        let mut state = ContainerMonitorState::default();
        let une = |i: usize| ContainerChangeEntry {
            timestamp: i.to_string(),
            kind: ContainerChangeKind::Added,
            container: snap("1", "img", "running"),
            previous: None,
        };
        for lot in 0..3 {
            let changes: Vec<_> = (0..100).map(|i| une(lot * 100 + i)).collect();
            state.apply("t".into(), vec![], changes);
        }
        assert_eq!(state.recent_changes.len(), MAX_RECENT_CHANGES);
        // Les plus ANCIENS sont oublies : le premier restant est le 100e emis.
        assert_eq!(state.recent_changes[0].timestamp, "100");
    }

    #[test]
    fn changement_etat_seul_sans_changement_image() {
        let avant = map(&[snap("1", "img:1", "running")]);
        let apres = map(&[snap("1", "img:1", "paused")]);
        let changes = detect_changes(&avant, &apres, "t");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, ContainerChangeKind::StateChanged);
    }

    #[test]
    fn as_action_pour_tous_les_types() {
        assert_eq!(ContainerChangeKind::Added.as_action(), "docker.added");
        assert_eq!(ContainerChangeKind::Removed.as_action(), "docker.removed");
        assert_eq!(ContainerChangeKind::ImageChanged.as_action(), "docker.image_changed");
        assert_eq!(ContainerChangeKind::StateChanged.as_action(), "docker.state_changed");
    }

    #[test]
    fn container_snapshot_creation() {
        let snap = snap("123", "nginx:latest", "running");
        assert_eq!(snap.id, "123");
        assert_eq!(snap.image, "nginx:latest");
        assert_eq!(snap.state, "running");
    }

    #[test]
    fn container_monitor_state_apply_updates_current() {
        let mut state = ContainerMonitorState::default();
        let snapshot = snap("1", "img", "running");
        state.apply("now".into(), vec![snapshot.clone()], vec![]);
        assert_eq!(state.current.len(), 1);
        assert_eq!(state.current[0].id, "1");
        assert_eq!(state.last_check, "now");
    }
}
