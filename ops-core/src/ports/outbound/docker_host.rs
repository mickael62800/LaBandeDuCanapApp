//! Port outbound : accès au daemon Docker de l'hôte (listing, actions,
//! prune, usage disque). Implémenté côté API par `BollardDockerHost`.

use async_trait::async_trait;

use crate::domain::entities::docker_host::{
    ContainerSummary, DiskUsage, DockerVersionInfo, ImageSummary, NetworkSummary, PruneOutcome,
    VolumeSummary,
};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait DockerHost: Send + Sync {
    /// Version + compteurs globaux du daemon (`version` + `info`).
    async fn version_info(&self) -> Result<DockerVersionInfo, DomainError>;

    /// Snapshot `docker system df` en types de domaine.
    async fn disk_usage(&self) -> Result<DiskUsage, DomainError>;

    // ── Containers ────────────────────────────────────────────────────────
    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>, DomainError>;
    async fn start_container(&self, id: &str) -> Result<(), DomainError>;
    async fn stop_container(&self, id: &str, timeout_secs: i64) -> Result<(), DomainError>;
    async fn restart_container(&self, id: &str, timeout_secs: i64) -> Result<(), DomainError>;
    async fn remove_container(
        &self,
        id: &str,
        force: bool,
        remove_volumes: bool,
    ) -> Result<(), DomainError>;

    /// Logs (stdout+stderr, sans follow), tronqués à ~2MB côté adapter.
    async fn container_logs(
        &self,
        id: &str,
        tail: u32,
        timestamps: bool,
    ) -> Result<String, DomainError>;

    // ── Images ────────────────────────────────────────────────────────────
    async fn list_images(&self) -> Result<Vec<ImageSummary>, DomainError>;
    async fn remove_image(&self, id: &str, force: bool, no_prune: bool) -> Result<(), DomainError>;

    // ── Volumes ───────────────────────────────────────────────────────────
    async fn list_volumes(&self) -> Result<Vec<VolumeSummary>, DomainError>;
    async fn remove_volume(&self, name: &str, force: bool) -> Result<(), DomainError>;

    // ── Networks ──────────────────────────────────────────────────────────
    async fn list_networks(&self) -> Result<Vec<NetworkSummary>, DomainError>;

    // ── Prune ─────────────────────────────────────────────────────────────
    async fn prune_containers(&self) -> Result<PruneOutcome, DomainError>;
    /// `all == true` → dangling=false (toutes les images inutilisées),
    /// sinon dangling=true (seulement sans tag). Même sémantique que la CLI.
    async fn prune_images(&self, all: bool) -> Result<PruneOutcome, DomainError>;
    async fn prune_volumes(&self) -> Result<PruneOutcome, DomainError>;
    async fn prune_networks(&self) -> Result<PruneOutcome, DomainError>;
    /// Purge du build cache (buildkit). `all == true` = tout le cache.
    async fn prune_build_cache(&self, all: bool) -> Result<PruneOutcome, DomainError>;
}
