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
use reqwest::Client;
use sentinel_core::domain::entities::ops::docker_host::{
    ContainerSummary, DiskUsage, DockerVersionInfo, ImageSummary, NetworkSummary, PruneOutcome,
    VolumeSummary,
};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::ops::docker_host::DockerHost;
use serde::de::DeserializeOwned;

pub struct HttpDockerHost {
    client: Client,
    base_url: String,
    token: String,
}

impl HttpDockerHost {
    /// `base_url` : `http://docker-agent:8095` en compose.
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            // Delai genereux : un `prune` d'images peut durer, la mesure du
            // `system df` aussi sur un hote charge. Trop court, l'ecran
            // afficherait une panne la ou l'operation se termine tres bien.
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            base_url,
            token,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    /// Traduit toute panne de transport en `DomainError`. L'agent tourne
    /// derriere un profil Docker optionnel : son absence est un cas normal
    /// d'installation, pas un bug, et le message doit le dire.
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
impl DockerHost for HttpDockerHost {
    async fn version_info(&self) -> Result<DockerVersionInfo, DomainError> {
        self.send(self.client.get(self.url("/version"))).await
    }

    async fn disk_usage(&self) -> Result<DiskUsage, DomainError> {
        self.send(self.client.get(self.url("/disk-usage"))).await
    }

    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>, DomainError> {
        self.send(
            self.client
                .get(self.url("/containers"))
                .query(&[("all", all)]),
        )
        .await
    }

    async fn start_container(&self, id: &str) -> Result<(), DomainError> {
        self.send_unit(
            self.client
                .post(self.url(&format!("/containers/{id}/start"))),
        )
        .await
    }

    async fn stop_container(&self, id: &str, timeout_secs: i64) -> Result<(), DomainError> {
        self.send_unit(
            self.client
                .post(self.url(&format!("/containers/{id}/stop")))
                .query(&[("timeout_secs", timeout_secs)]),
        )
        .await
    }

    async fn restart_container(&self, id: &str, timeout_secs: i64) -> Result<(), DomainError> {
        self.send_unit(
            self.client
                .post(self.url(&format!("/containers/{id}/restart")))
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
            self.client
                .post(self.url(&format!("/containers/{id}/remove")))
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
        let response = self
            .client
            .get(self.url(&format!("/containers/{id}/logs")))
            .query(&[("tail", tail.to_string()), ("timestamps", timestamps.to_string())])
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| {
                tracing::warn!(%error, "docker-agent injoignable");
                DomainError::Internal("docker-agent injoignable".into())
            })?;

        if !response.status().is_success() {
            return Err(DomainError::Internal(format!(
                "docker-agent: reponse {}",
                response.status()
            )));
        }

        // Les logs sont du texte brut, pas du JSON : les passer par `json()`
        // les re-encoderait inutilement.
        response.text().await.map_err(|error| {
            tracing::warn!(%error, "lecture des logs impossible");
            DomainError::Internal("lecture des logs impossible".into())
        })
    }

    async fn list_images(&self) -> Result<Vec<ImageSummary>, DomainError> {
        self.send(self.client.get(self.url("/images"))).await
    }

    async fn remove_image(&self, id: &str, force: bool, no_prune: bool) -> Result<(), DomainError> {
        self.send_unit(
            self.client
                .post(self.url(&format!("/images/{id}/remove")))
                .query(&[("force", force), ("no_prune", no_prune)]),
        )
        .await
    }

    async fn list_volumes(&self) -> Result<Vec<VolumeSummary>, DomainError> {
        self.send(self.client.get(self.url("/volumes"))).await
    }

    async fn remove_volume(&self, name: &str, force: bool) -> Result<(), DomainError> {
        self.send_unit(
            self.client
                .post(self.url(&format!("/volumes/{name}/remove")))
                .query(&[("force", force)]),
        )
        .await
    }

    async fn list_networks(&self) -> Result<Vec<NetworkSummary>, DomainError> {
        self.send(self.client.get(self.url("/networks"))).await
    }

    async fn prune_containers(&self) -> Result<PruneOutcome, DomainError> {
        self.send(self.client.post(self.url("/prune/containers")))
            .await
    }

    async fn prune_images(&self, all: bool) -> Result<PruneOutcome, DomainError> {
        self.send(
            self.client
                .post(self.url("/prune/images"))
                .query(&[("all", all)]),
        )
        .await
    }

    async fn prune_volumes(&self) -> Result<PruneOutcome, DomainError> {
        self.send(self.client.post(self.url("/prune/volumes")))
            .await
    }

    async fn prune_networks(&self) -> Result<PruneOutcome, DomainError> {
        self.send(self.client.post(self.url("/prune/networks")))
            .await
    }

    async fn prune_build_cache(&self, all: bool) -> Result<PruneOutcome, DomainError> {
        self.send(
            self.client
                .post(self.url("/prune/build-cache"))
                .query(&[("all", all)]),
        )
        .await
    }
}
