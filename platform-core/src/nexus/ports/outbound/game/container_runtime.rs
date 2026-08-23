//! Cycle de vie des conteneurs de jeu — réexportation du contexte Ops.
//!
//! # Pourquoi le port n'est plus défini ici
//!
//! Ces types décrivent une opération sur le daemon Docker de l'hôte, pas une
//! règle du portail de jeux. Les héberger dans `nexus-core` avait une
//! conséquence concrète : `nexus-api` implémentait le port avec `bollard` et
//! montait donc `/var/run/docker.sock` — un équivalent root sur l'hôte dans le
//! processus qui sert aussi les routes publiques du portail. C'est exactement
//! ce que `sentinel-api` avait déjà cessé de faire en passant par
//! `docker-agent`.
//!
//! Le port vit maintenant dans `platform_core::ops` (domaine neutre de la machine hôte),
//! `docker-agent` l'implémente une fois avec bollard, et `nexus-api` n'en garde
//! qu'un client HTTP. Ce module reste la porte d'entrée pour tout Nexus : les
//! use cases et leurs tests n'ont pas bougé.
//!
//! `ContainerRuntime` est un alias de `GameContainerRuntime` — le nom court
//! reste celui qu'emploie le domaine Nexus, où il n'y a pas d'ambiguïté.

use async_trait::async_trait;
use std::collections::HashMap;

pub use crate::ops::domain::entities::game_runtime::{
    ContainerSpec, ContainerState, ContainerStats, ContainerStatus, ManagedContainer, PortMapping,
    PortProtocol, RestartPolicy, VolumeArchive, VolumeMount,
};
pub use crate::ops::ports::outbound::game_runtime::GameContainerRuntime as ContainerRuntime;

use crate::nexus::domain::errors::DomainError;
/// Implementer un Mock Container Runtime en memoire pour les tests et le dev local sans Docker.
use std::sync::Mutex;

#[derive(Default)]
pub struct MockContainerRuntime {
    containers: Mutex<HashMap<String, ContainerStatus>>,
}

impl MockContainerRuntime {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ContainerRuntime for MockContainerRuntime {
    fn is_operational(&self) -> bool {
        true
    }

    async fn ensure_network(&self, _name: &str) -> Result<(), DomainError> {
        Ok(())
    }

    async fn ensure_volume(&self, _name: &str) -> Result<(), DomainError> {
        Ok(())
    }

    async fn archive_volume(
        &self,
        _volume: &str,
        nom_fichier: &str,
    ) -> Result<VolumeArchive, DomainError> {
        Ok(VolumeArchive {
            path: format!("/backup/{nom_fichier}"),
            size_bytes: 0,
        })
    }

    async fn pull_image_if_missing(&self, _image: &str) -> Result<(), DomainError> {
        Ok(())
    }

    async fn create_container(&self, spec: &ContainerSpec) -> Result<String, DomainError> {
        let id = format!("mock-{}", spec.name);
        let status = ContainerStatus {
            container_id: id.clone(),
            state: ContainerState::Created,
            exit_code: None,
            error: None,
        };
        self.containers.lock().unwrap().insert(id.clone(), status);
        Ok(id)
    }

    async fn start_container(&self, container_id: &str) -> Result<(), DomainError> {
        let mut guard = self.containers.lock().unwrap();
        if let Some(status) = guard.get_mut(container_id) {
            status.state = ContainerState::Running;
        }
        Ok(())
    }

    async fn upload_file_to_container(
        &self,
        _container_id: &str,
        _path: &str,
        _content: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn stop_container(
        &self,
        container_id: &str,
        _timeout_secs: u32,
    ) -> Result<(), DomainError> {
        let mut guard = self.containers.lock().unwrap();
        if let Some(status) = guard.get_mut(container_id) {
            status.state = ContainerState::Exited;
            status.exit_code = Some(0);
        }
        Ok(())
    }

    async fn restart_container(
        &self,
        container_id: &str,
        _timeout_secs: u32,
    ) -> Result<(), DomainError> {
        self.stop_container(container_id, 5).await?;
        self.start_container(container_id).await
    }

    async fn remove_container(&self, container_id: &str) -> Result<(), DomainError> {
        self.containers.lock().unwrap().remove(container_id);
        Ok(())
    }

    async fn remove_volume(&self, _name: &str) -> Result<(), DomainError> {
        Ok(())
    }

    async fn remove_image(&self, _image: &str, _force: bool) -> Result<bool, DomainError> {
        Ok(true)
    }

    async fn inspect(&self, container_id: &str) -> Result<Option<ContainerStatus>, DomainError> {
        let guard = self.containers.lock().unwrap();
        Ok(guard.get(container_id).cloned())
    }

    async fn stats(&self, _container_id: &str) -> Result<ContainerStats, DomainError> {
        Ok(ContainerStats {
            cpu_percent: 2.5,
            memory_used_bytes: 512 * 1024 * 1024,
            memory_limit_bytes: 2048 * 1024 * 1024,
            network_rx_bytes: 102400,
            network_tx_bytes: 204800,
        })
    }

    async fn logs(&self, _container_id: &str, _lines: u32) -> Result<Vec<String>, DomainError> {
        Ok(vec![
            "[Mock Runtime] Server initialized successfully.".into()
        ])
    }

    async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError> {
        let guard = self.containers.lock().unwrap();
        Ok(guard
            .iter()
            .map(|(id, status)| ManagedContainer {
                container_id: id.clone(),
                name: id.clone(),
                state: status.state,
                labels: HashMap::new(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_container_runtime_lifecycle() {
        let runtime = MockContainerRuntime::new();
        assert!(runtime.is_operational());

        let spec = ContainerSpec {
            image: "minecraft:latest".into(),
            name: "test-mc".into(),
            env: HashMap::new(),
            port_mappings: vec![],
            volumes: vec![],
            memory_bytes: 1024 * 1024 * 1024,
            cpu_limit: None,
            network: "nexus-net".into(),
            user: None,
            restart_policy: RestartPolicy::None,
            labels: HashMap::new(),
            command: None,
        };

        let id = runtime.create_container(&spec).await.unwrap();
        let inspect_created = runtime.inspect(&id).await.unwrap().unwrap();
        assert_eq!(inspect_created.state, ContainerState::Created);

        runtime.start_container(&id).await.unwrap();
        let inspect_running = runtime.inspect(&id).await.unwrap().unwrap();
        assert_eq!(inspect_running.state, ContainerState::Running);

        runtime.stop_container(&id, 5).await.unwrap();
        let inspect_stopped = runtime.inspect(&id).await.unwrap().unwrap();
        assert_eq!(inspect_stopped.state, ContainerState::Exited);

        runtime.remove_container(&id).await.unwrap();
        assert!(runtime.inspect(&id).await.unwrap().is_none());
    }
}
// Port vers le moteur d'exécution des serveurs de jeu. Le domaine demande des
// opérations abstraites ; l'API choisit Docker, un agent distant ou noop.
