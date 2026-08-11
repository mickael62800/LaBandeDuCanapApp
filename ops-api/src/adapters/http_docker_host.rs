//! Adapter sortant : daemon Docker via le `docker-agent`.
//!
//! # Pourquoi un appel reseau plutot que le socket
//!
//! `/var/run/docker.sock` equivaut a un acces root sur l'hote. Il etait monte
//! par ce processus, qui sert aussi l'OAuth, la moderation Discord et toutes
//! les routes communautaires : la moindre faille dans cette surface donnait
//! l'hote. Le socket n'est plus monte que par `docker-agent`, un service sans
//! base et sans utilisateurs, joignable uniquement sur le reseau interne.
//!
//! Le port `DockerHost` n'a pas change : les handlers ignorent que Docker est
//! passe de l'autre cote d'un appel HTTP. C'est precisement l'interet d'avoir
//! eu un port plutot qu'un client bollard appele directement.
//!
//! L'implementation bollard n'a pas ete dupliquee : elle a ete DEPLACEE dans
//! `docker-agent/src/bollard_host.rs`. Il n'existe toujours qu'un seul mapping
//! bollard -> domaine dans le depot.

use async_trait::async_trait;
use ops_core::domain::entities::docker_host::{
    ContainerSummary, DiskUsage, DockerVersionInfo, ImageSummary, NetworkSummary, PruneOutcome,
    VolumeSummary,
};
use ops_core::domain::errors::DomainError;
use ops_core::ports::outbound::docker_host::DockerHost;
use platform_common_api::docker_agent_client::{DockerAgentClient, DockerAgentError};
use serde::de::DeserializeOwned;

pub struct HttpDockerHost {
    agent: DockerAgentClient,
}

impl HttpDockerHost {
    /// `base_url` : `http://docker-agent:8095` en compose.
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            agent: DockerAgentClient::new(base_url, token, std::time::Duration::from_secs(120)),
        }
    }

    fn map_error(error: DockerAgentError) -> DomainError {
        match &error {
            DockerAgentError::Transport(source) => {
                tracing::warn!(error = %source, "docker-agent injoignable")
            }
            DockerAgentError::Rejected { status, detail } => {
                tracing::warn!(%status, body = %detail, "docker-agent a refuse l'operation")
            }
            DockerAgentError::InvalidResponse(source) => {
                tracing::warn!(error = %source, "reponse docker-agent illisible")
            }
        }
        DomainError::Internal(error.to_string())
    }

    /// Traduit toute panne de transport en `DomainError`. L'agent tourne
    /// derriere un profil Docker optionnel : son absence est un cas normal
    /// d'installation, pas un bug, et le message doit le dire.
    async fn send<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, DomainError> {
        self.agent.send_json(request).await.map_err(Self::map_error)
    }

    /// Variante pour les operations sans corps de reponse (204).
    async fn send_unit(&self, request: reqwest::RequestBuilder) -> Result<(), DomainError> {
        self.agent.send_unit(request).await.map_err(Self::map_error)
    }
}

#[async_trait]
impl DockerHost for HttpDockerHost {
    async fn version_info(&self) -> Result<DockerVersionInfo, DomainError> {
        self.send(self.agent.get("/version")).await
    }

    async fn disk_usage(&self) -> Result<DiskUsage, DomainError> {
        self.send(self.agent.get("/disk-usage")).await
    }

    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>, DomainError> {
        self.send(self.agent.get("/containers").query(&[("all", all)]))
            .await
    }

    async fn start_container(&self, id: &str) -> Result<(), DomainError> {
        self.send_unit(self.agent.post(&format!("/containers/{id}/start")))
            .await
    }

    async fn stop_container(&self, id: &str, timeout_secs: i64) -> Result<(), DomainError> {
        self.send_unit(
            self.agent
                .post(&format!("/containers/{id}/stop"))
                .query(&[("timeout_secs", timeout_secs)]),
        )
        .await
    }

    async fn restart_container(&self, id: &str, timeout_secs: i64) -> Result<(), DomainError> {
        self.send_unit(
            self.agent
                .post(&format!("/containers/{id}/restart"))
                .query(&[("timeout_secs", timeout_secs)]),
        )
        .await
    }

    async fn remove_container(
        &self,
        id: &str,
        force: bool,
        remove_volumes: bool,
    ) -> Result<(), DomainError> {
        self.send_unit(
            self.agent
                .post(&format!("/containers/{id}/remove"))
                .query(&[("force", force), ("remove_volumes", remove_volumes)]),
        )
        .await
    }

    async fn container_logs(
        &self,
        id: &str,
        tail: u32,
        timestamps: bool,
    ) -> Result<String, DomainError> {
        self.agent
            .send_text(self.agent.get(&format!("/containers/{id}/logs")).query(&[
                ("tail", tail.to_string()),
                ("timestamps", timestamps.to_string()),
            ]))
            .await
            .map_err(Self::map_error)
    }

    async fn list_images(&self) -> Result<Vec<ImageSummary>, DomainError> {
        self.send(self.agent.get("/images")).await
    }

    async fn remove_image(&self, id: &str, force: bool, no_prune: bool) -> Result<(), DomainError> {
        self.send_unit(
            self.agent
                .post(&format!("/images/{id}/remove"))
                .query(&[("force", force), ("no_prune", no_prune)]),
        )
        .await
    }

    async fn list_volumes(&self) -> Result<Vec<VolumeSummary>, DomainError> {
        self.send(self.agent.get("/volumes")).await
    }

    async fn remove_volume(&self, name: &str, force: bool) -> Result<(), DomainError> {
        self.send_unit(
            self.agent
                .post(&format!("/volumes/{name}/remove"))
                .query(&[("force", force)]),
        )
        .await
    }

    async fn list_networks(&self) -> Result<Vec<NetworkSummary>, DomainError> {
        self.send(self.agent.get("/networks")).await
    }

    async fn prune_containers(&self) -> Result<PruneOutcome, DomainError> {
        self.send(self.agent.post("/prune/containers")).await
    }

    async fn prune_images(&self, all: bool) -> Result<PruneOutcome, DomainError> {
        self.send(self.agent.post("/prune/images").query(&[("all", all)]))
            .await
    }

    async fn prune_volumes(&self) -> Result<PruneOutcome, DomainError> {
        self.send(self.agent.post("/prune/volumes")).await
    }

    async fn prune_networks(&self) -> Result<PruneOutcome, DomainError> {
        self.send(self.agent.post("/prune/networks")).await
    }

    async fn prune_build_cache(&self, all: bool) -> Result<PruneOutcome, DomainError> {
        self.send(self.agent.post("/prune/build-cache").query(&[("all", all)]))
            .await
    }
}
