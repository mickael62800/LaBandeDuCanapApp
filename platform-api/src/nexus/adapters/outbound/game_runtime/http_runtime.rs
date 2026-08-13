//! Adapter sortant : cycle de vie des conteneurs de jeu via le `docker-agent`.
//!
//! # Pourquoi un appel reseau plutot que le socket
//!
//! `/var/run/docker.sock` equivaut a un acces root sur l'hote. Il etait monte
//! par ce processus, qui sert aussi la vitrine publique du portail et les
//! routes d'administration des serveurs : la moindre faille dans cette surface
//! donnait l'hote. `sentinel-api` avait deja fait ce chemin ; Nexus etait reste
//! en arriere, avec en prime un SECOND mapping bollard -> domaine dans le
//! depot.
//!
//! Le port n'a pas changé de forme : les cas d'usage Nexus ignorent que
//! Docker est passe de l'autre cote d'un appel HTTP. C'est tout l'interet
//! d'avoir eu un port plutot qu'un client bollard appele directement.
//!
//! L'implementation bollard n'a pas ete dupliquee : elle a ete DEPLACEE dans
//! `docker-agent/src/bollard_game.rs`. Il n'existe toujours qu'un seul mapping
//! bollard -> domaine dans le depot.

use async_trait::async_trait;
use ops_adapters::docker_agent_client::{DockerAgentClient, DockerAgentError};
use serde::de::DeserializeOwned;

use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::container_runtime::{
    ContainerRuntime, ContainerSpec, ContainerStats, ContainerStatus, ManagedContainer,
};

pub struct HttpGameRuntime {
    agent: DockerAgentClient,
    /// Fige au demarrage par un appel a `/game/operational`.
    ///
    /// `is_operational` est synchrone dans le port (il sert a REFUSER une
    /// creation d'emblee, avant tout travail) : on ne peut donc pas interroger
    /// l'agent a chaque appel. Un agent qui perd son socket apres le boot fera
    /// echouer les operations une par une, ce qui reste le bon comportement —
    /// simplement moins precoce.
    operational: bool,
}

impl HttpGameRuntime {
    /// `base_url` : `http://docker-agent:8095` en compose.
    ///
    /// La sonde initiale est volontairement tolerante : si l'agent ne repond
    /// pas encore au boot, on se declare non operationnel plutot que de faire
    /// echouer le demarrage de l'API. Le portail reste consultable, seules les
    /// operations de cycle de vie sont refusees — exactement ce que faisait le
    /// repli `noop` quand le socket etait absent.
    pub async fn connect(base_url: String, token: String) -> Self {
        let agent =
            DockerAgentClient::new(base_url.clone(), token, std::time::Duration::from_secs(600));
        let operational = agent
            .probe("/game/operational", std::time::Duration::from_secs(5))
            .await;

        if !operational {
            tracing::warn!(
                %base_url,
                "docker-agent injoignable ou sans socket : cycle de vie des serveurs indisponible"
            );
        }

        Self { agent, operational }
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
impl ContainerRuntime for HttpGameRuntime {
    fn is_operational(&self) -> bool {
        self.operational
    }

    async fn ensure_network(&self, name: &str) -> Result<(), DomainError> {
        self.send_unit(self.agent.post(&format!("/game/networks/{name}/ensure")))
            .await
    }

    async fn ensure_volume(&self, name: &str) -> Result<(), DomainError> {
        self.send_unit(self.agent.post(&format!("/game/volumes/{name}/ensure")))
            .await
    }

    async fn pull_image_if_missing(&self, image: &str) -> Result<(), DomainError> {
        self.send_unit(
            self.agent
                .post("/game/images/pull")
                .json(&serde_json::json!({ "image": image })),
        )
        .await
    }

    async fn create_container(&self, spec: &ContainerSpec) -> Result<String, DomainError> {
        self.send(self.agent.post("/game/containers").json(spec))
            .await
    }

    async fn start_container(&self, container_id: &str) -> Result<(), DomainError> {
        self.send_unit(
            self.agent
                .post(&format!("/game/containers/{container_id}/start")),
        )
        .await
    }

    async fn upload_file_to_container(
        &self,
        container_id: &str,
        path: &str,
        content: &str,
    ) -> Result<(), DomainError> {
        self.send_unit(
            self.agent
                .post(&format!("/game/containers/{container_id}/upload"))
                .json(&serde_json::json!({ "path": path, "content": content })),
        )
        .await
    }

    async fn stop_container(
        &self,
        container_id: &str,
        timeout_secs: u32,
    ) -> Result<(), DomainError> {
        self.send_unit(
            self.agent
                .post(&format!("/game/containers/{container_id}/stop"))
                .query(&[("timeout_secs", timeout_secs)]),
        )
        .await
    }

    async fn restart_container(
        &self,
        container_id: &str,
        timeout_secs: u32,
    ) -> Result<(), DomainError> {
        self.send_unit(
            self.agent
                .post(&format!("/game/containers/{container_id}/restart"))
                .query(&[("timeout_secs", timeout_secs)]),
        )
        .await
    }

    async fn remove_container(&self, container_id: &str) -> Result<(), DomainError> {
        self.send_unit(
            self.agent
                .post(&format!("/game/containers/{container_id}/remove")),
        )
        .await
    }

    async fn remove_volume(&self, name: &str) -> Result<(), DomainError> {
        self.send_unit(self.agent.post(&format!("/game/volumes/{name}/remove")))
            .await
    }

    async fn remove_image(&self, image: &str, force: bool) -> Result<bool, DomainError> {
        self.send(
            self.agent
                .post("/game/images/remove")
                .json(&serde_json::json!({ "image": image, "force": force })),
        )
        .await
    }

    async fn inspect(&self, container_id: &str) -> Result<Option<ContainerStatus>, DomainError> {
        self.send(
            self.agent
                .get(&format!("/game/containers/{container_id}/inspect")),
        )
        .await
    }

    async fn stats(&self, container_id: &str) -> Result<ContainerStats, DomainError> {
        self.send(
            self.agent
                .get(&format!("/game/containers/{container_id}/stats")),
        )
        .await
    }

    async fn logs(&self, container_id: &str, lines: u32) -> Result<Vec<String>, DomainError> {
        self.send(
            self.agent
                .get(&format!("/game/containers/{container_id}/logs"))
                .query(&[("lines", lines)]),
        )
        .await
    }

    async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError> {
        self.send(self.agent.get("/game/containers/managed")).await
    }
}
