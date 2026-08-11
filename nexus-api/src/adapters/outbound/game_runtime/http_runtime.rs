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
//! Le port n'a pas change de forme : les use cases de `nexus-core` ignorent que
//! Docker est passe de l'autre cote d'un appel HTTP. C'est tout l'interet
//! d'avoir eu un port plutot qu'un client bollard appele directement.
//!
//! L'implementation bollard n'a pas ete dupliquee : elle a ete DEPLACEE dans
//! `docker-agent/src/bollard_game.rs`. Il n'existe toujours qu'un seul mapping
//! bollard -> domaine dans le depot.

use async_trait::async_trait;
use reqwest::Client;
use serde::de::DeserializeOwned;

use nexus_core::domain::errors::DomainError;
use nexus_core::ports::outbound::game::container_runtime::{
    ContainerRuntime, ContainerSpec, ContainerStats, ContainerStatus, ManagedContainer,
};

pub struct HttpGameRuntime {
    client: Client,
    base_url: String,
    token: String,
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
        let client = Client::builder()
            // Genereux : un `pull` d'image de serveur de jeu se compte en
            // minutes. Trop court, l'API declarerait en panne une creation qui
            // se termine tres bien.
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .unwrap_or_default();

        let operational = client
            .get(format!(
                "{}/game/operational",
                base_url.trim_end_matches('/')
            ))
            .bearer_auth(&token)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .ok()
            .filter(|r| r.status().is_success())
            .is_some();

        if !operational {
            tracing::warn!(
                %base_url,
                "docker-agent injoignable ou sans socket : cycle de vie des serveurs indisponible"
            );
        }

        Self {
            client,
            base_url,
            token,
            operational,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    async fn send<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, DomainError> {
        let response = request
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| {
                tracing::warn!(%error, "docker-agent injoignable");
                DomainError::Internal("docker-agent injoignable".into())
            })?;

        let status = response.status();
        if !status.is_success() {
            let detail = response.text().await.unwrap_or_default();
            tracing::warn!(%status, body = %detail, "docker-agent a refuse l'operation");
            return Err(DomainError::Internal(format!(
                "docker-agent: reponse {status}"
            )));
        }

        response.json::<T>().await.map_err(|error| {
            tracing::warn!(%error, "reponse docker-agent illisible");
            DomainError::Internal("reponse docker-agent illisible".into())
        })
    }

    /// Variante pour les operations sans corps de reponse (204).
    async fn send_unit(&self, request: reqwest::RequestBuilder) -> Result<(), DomainError> {
        let response = request
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| {
                tracing::warn!(%error, "docker-agent injoignable");
                DomainError::Internal("docker-agent injoignable".into())
            })?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let detail = response.text().await.unwrap_or_default();
            tracing::warn!(%status, body = %detail, "docker-agent a refuse l'operation");
            Err(DomainError::Internal(format!(
                "docker-agent: reponse {status}"
            )))
        }
    }
}

#[async_trait]
impl ContainerRuntime for HttpGameRuntime {
    fn is_operational(&self) -> bool {
        self.operational
    }

    async fn ensure_network(&self, name: &str) -> Result<(), DomainError> {
        self.send_unit(
            self.client
                .post(self.url(&format!("/game/networks/{name}/ensure"))),
        )
        .await
    }

    async fn ensure_volume(&self, name: &str) -> Result<(), DomainError> {
        self.send_unit(
            self.client
                .post(self.url(&format!("/game/volumes/{name}/ensure"))),
        )
        .await
    }

    async fn pull_image_if_missing(&self, image: &str) -> Result<(), DomainError> {
        self.send_unit(
            self.client
                .post(self.url("/game/images/pull"))
                .json(&serde_json::json!({ "image": image })),
        )
        .await
    }

    async fn create_container(&self, spec: &ContainerSpec) -> Result<String, DomainError> {
        self.send(self.client.post(self.url("/game/containers")).json(spec))
            .await
    }

    async fn start_container(&self, container_id: &str) -> Result<(), DomainError> {
        self.send_unit(
            self.client
                .post(self.url(&format!("/game/containers/{container_id}/start"))),
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
            self.client
                .post(self.url(&format!("/game/containers/{container_id}/upload")))
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
            self.client
                .post(self.url(&format!("/game/containers/{container_id}/stop")))
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
            self.client
                .post(self.url(&format!("/game/containers/{container_id}/restart")))
                .query(&[("timeout_secs", timeout_secs)]),
        )
        .await
    }

    async fn remove_container(&self, container_id: &str) -> Result<(), DomainError> {
        self.send_unit(
            self.client
                .post(self.url(&format!("/game/containers/{container_id}/remove"))),
        )
        .await
    }

    async fn remove_volume(&self, name: &str) -> Result<(), DomainError> {
        self.send_unit(
            self.client
                .post(self.url(&format!("/game/volumes/{name}/remove"))),
        )
        .await
    }

    async fn remove_image(&self, image: &str, force: bool) -> Result<bool, DomainError> {
        self.send(
            self.client
                .post(self.url("/game/images/remove"))
                .json(&serde_json::json!({ "image": image, "force": force })),
        )
        .await
    }

    async fn inspect(&self, container_id: &str) -> Result<Option<ContainerStatus>, DomainError> {
        self.send(
            self.client
                .get(self.url(&format!("/game/containers/{container_id}/inspect"))),
        )
        .await
    }

    async fn stats(&self, container_id: &str) -> Result<ContainerStats, DomainError> {
        self.send(
            self.client
                .get(self.url(&format!("/game/containers/{container_id}/stats"))),
        )
        .await
    }

    async fn logs(&self, container_id: &str, lines: u32) -> Result<Vec<String>, DomainError> {
        self.send(
            self.client
                .get(self.url(&format!("/game/containers/{container_id}/logs")))
                .query(&[("lines", lines)]),
        )
        .await
    }

    async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError> {
        self.send(self.client.get(self.url("/game/containers/managed")))
            .await
    }
}
