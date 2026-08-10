//! Adapter sortant : daemon Docker via bollard (socket /var/run/docker.sock).
//!
//! Implémente le port `DockerHost` du core. Tout le mapping bollard → types
//! de domaine vit ici ; les handlers HTTP ne voient plus bollard.

use std::collections::HashMap;
use std::sync::OnceLock;

use async_trait::async_trait;
use bollard::container::ListContainersOptions;
use bollard::container::LogsOptions;
use bollard::container::RemoveContainerOptions;
use bollard::container::RestartContainerOptions;
use bollard::container::StopContainerOptions;
use bollard::image::ListImagesOptions;
use bollard::image::RemoveImageOptions;
use bollard::network::ListNetworksOptions;
use bollard::volume::ListVolumesOptions;
use bollard::Docker;
use futures_util::StreamExt;

use ops_core::domain::entities::docker_host::{
    BuildCacheEntry, ContainerDiskUsage, ContainerPort, ContainerSummary, DiskUsage,
    DockerVersionInfo, ImageDiskUsage, ImageSummary, NetworkSummary, PruneOutcome, VolumeDiskUsage,
    VolumeSummary,
};
use ops_core::domain::errors::DomainError;
use ops_core::ports::outbound::docker_host::DockerHost;

/// Singleton du client Docker. Bollard ouvre une connexion lazy au socket.
static DOCKER: OnceLock<Docker> = OnceLock::new();

fn docker() -> Result<&'static Docker, DomainError> {
    if let Some(d) = DOCKER.get() {
        return Ok(d);
    }
    let d = Docker::connect_with_local_defaults()
        .map_err(|e| DomainError::Internal(format!("docker socket: {}", e)))?;
    let _ = DOCKER.set(d);
    Ok(DOCKER.get().expect("docker just initialized"))
}

fn map_err(e: bollard::errors::Error) -> DomainError {
    DomainError::Internal(format!("docker: {}", e))
}

/// Implémentation bollard du port `DockerHost`.
pub struct BollardDockerHost;

#[async_trait]
impl DockerHost for BollardDockerHost {
    async fn version_info(&self) -> Result<DockerVersionInfo, DomainError> {
        let d = docker()?;
        let v = d.version().await.map_err(map_err)?;
        let info = d.info().await.map_err(map_err)?;
        Ok(DockerVersionInfo {
            version: v.version.unwrap_or_default(),
            api_version: v.api_version.unwrap_or_default(),
            os: v.os.unwrap_or_default(),
            arch: v.arch.unwrap_or_default(),
            kernel: v.kernel_version.unwrap_or_default(),
            containers_running: info.containers_running.unwrap_or(0),
            containers_paused: info.containers_paused.unwrap_or(0),
            containers_stopped: info.containers_stopped.unwrap_or(0),
            images_count: info.images.unwrap_or(0),
        })
    }

    async fn disk_usage(&self) -> Result<DiskUsage, DomainError> {
        let d = docker()?;
        let df = d.df().await.map_err(map_err)?;
        Ok(DiskUsage {
            layers_size: df.layers_size.unwrap_or(0),
            images: df
                .images
                .unwrap_or_default()
                .into_iter()
                .map(|i| ImageDiskUsage {
                    size: i.size,
                    containers: i.containers,
                })
                .collect(),
            containers: df
                .containers
                .unwrap_or_default()
                .into_iter()
                .map(|c| ContainerDiskUsage {
                    size_rw: c.size_rw.unwrap_or(0),
                    state: c.state,
                })
                .collect(),
            volumes: df
                .volumes
                .unwrap_or_default()
                .into_iter()
                .map(|v| match v.usage_data {
                    Some(u) => VolumeDiskUsage {
                        size: Some(u.size),
                        ref_count: Some(u.ref_count),
                    },
                    None => VolumeDiskUsage {
                        size: None,
                        ref_count: None,
                    },
                })
                .collect(),
            build_cache: df
                .build_cache
                .unwrap_or_default()
                .into_iter()
                .map(|c| BuildCacheEntry {
                    size: c.size.unwrap_or(0),
                    in_use: c.in_use.unwrap_or(false),
                })
                .collect(),
        })
    }

    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>, DomainError> {
        let d = docker()?;
        let opts = ListContainersOptions::<String> {
            all,
            size: true,
            ..Default::default()
        };
        let list = d.list_containers(Some(opts)).await.map_err(map_err)?;
        Ok(list
            .into_iter()
            .map(|c| ContainerSummary {
                id: c.id.unwrap_or_default(),
                names: c.names.unwrap_or_default(),
                image: c.image.unwrap_or_default(),
                state: c.state.unwrap_or_default(),
                status: c.status.unwrap_or_default(),
                created: c.created.unwrap_or(0),
                size_rw: c.size_rw,
                size_root_fs: c.size_root_fs,
                ports: c
                    .ports
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| ContainerPort {
                        private_port: p.private_port as i64,
                        public_port: p.public_port.filter(|&pp| pp > 0).map(|pp| pp as i64),
                        protocol: p
                            .typ
                            .map(|t| format!("{:?}", t).to_lowercase())
                            .unwrap_or_else(|| "tcp".to_string()),
                    })
                    .collect(),
                labels: c.labels.unwrap_or_default(),
            })
            .collect())
    }

    async fn start_container(&self, id: &str) -> Result<(), DomainError> {
        let d = docker()?;
        d.start_container::<String>(id, None).await.map_err(map_err)
    }

    async fn stop_container(&self, id: &str, timeout_secs: i64) -> Result<(), DomainError> {
        let d = docker()?;
        let opts = StopContainerOptions { t: timeout_secs };
        d.stop_container(id, Some(opts)).await.map_err(map_err)
    }

    async fn restart_container(&self, id: &str, timeout_secs: i64) -> Result<(), DomainError> {
        let d = docker()?;
        let opts = RestartContainerOptions {
            t: timeout_secs as isize,
        };
        d.restart_container(id, Some(opts)).await.map_err(map_err)
    }

    async fn remove_container(
        &self,
        id: &str,
        force: bool,
        remove_volumes: bool,
    ) -> Result<(), DomainError> {
        let d = docker()?;
        let opts = RemoveContainerOptions {
            force,
            v: remove_volumes,
            ..Default::default()
        };
        d.remove_container(id, Some(opts)).await.map_err(map_err)
    }

    async fn container_logs(
        &self,
        id: &str,
        tail: u32,
        timestamps: bool,
    ) -> Result<String, DomainError> {
        let d = docker()?;
        let opts = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail.to_string(),
            timestamps,
            follow: false,
            ..Default::default()
        };
        let mut stream = d.logs(id, Some(opts));
        let mut out = String::new();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(c) => out.push_str(&c.to_string()),
                Err(e) => return Err(map_err(e)),
            }
            if out.len() > 2_000_000 {
                out.push_str("\n[...troncature 2MB...]");
                break;
            }
        }
        Ok(out)
    }

    async fn list_images(&self) -> Result<Vec<ImageSummary>, DomainError> {
        let d = docker()?;
        let opts = ListImagesOptions::<String> {
            all: false,
            ..Default::default()
        };
        let list = d.list_images(Some(opts)).await.map_err(map_err)?;
        Ok(list
            .into_iter()
            .map(|i| ImageSummary {
                id: i.id,
                repo_tags: i.repo_tags,
                repo_digests: i.repo_digests,
                created: i.created,
                size: i.size,
                shared_size: i.shared_size,
                virtual_size: i.virtual_size.unwrap_or(0),
                containers: i.containers,
            })
            .collect())
    }

    async fn remove_image(&self, id: &str, force: bool, no_prune: bool) -> Result<(), DomainError> {
        let d = docker()?;
        let opts = RemoveImageOptions {
            force,
            noprune: no_prune,
        };
        d.remove_image(id, Some(opts), None)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn list_volumes(&self) -> Result<Vec<VolumeSummary>, DomainError> {
        let d = docker()?;
        let resp = d
            .list_volumes(None::<ListVolumesOptions<String>>)
            .await
            .map_err(map_err)?;
        Ok(resp
            .volumes
            .unwrap_or_default()
            .into_iter()
            .map(|v| {
                let (size, ref_count) = match &v.usage_data {
                    Some(u) => (Some(u.size), Some(u.ref_count)),
                    None => (None, None),
                };
                VolumeSummary {
                    name: v.name,
                    driver: v.driver,
                    mountpoint: v.mountpoint,
                    created_at: v.created_at,
                    size,
                    ref_count,
                }
            })
            .collect())
    }

    async fn remove_volume(&self, name: &str, force: bool) -> Result<(), DomainError> {
        let d = docker()?;
        let opts = bollard::volume::RemoveVolumeOptions { force };
        d.remove_volume(name, Some(opts)).await.map_err(map_err)
    }

    async fn list_networks(&self) -> Result<Vec<NetworkSummary>, DomainError> {
        let d = docker()?;
        let list = d
            .list_networks(None::<ListNetworksOptions<String>>)
            .await
            .map_err(map_err)?;
        Ok(list
            .into_iter()
            .map(|n| NetworkSummary {
                id: n.id.unwrap_or_default(),
                name: n.name.unwrap_or_default(),
                driver: n.driver.unwrap_or_default(),
                scope: n.scope.unwrap_or_default(),
                internal: n.internal.unwrap_or(false),
                containers_count: n.containers.map(|c| c.len()).unwrap_or(0),
            })
            .collect())
    }

    async fn prune_containers(&self) -> Result<PruneOutcome, DomainError> {
        let d = docker()?;
        let r = d
            .prune_containers(None::<bollard::container::PruneContainersOptions<String>>)
            .await
            .map_err(map_err)?;
        Ok(PruneOutcome {
            deleted: r.containers_deleted.unwrap_or_default(),
            space_reclaimed_bytes: r.space_reclaimed.unwrap_or(0) as u64,
        })
    }

    async fn prune_images(&self, all: bool) -> Result<PruneOutcome, DomainError> {
        let d = docker()?;
        let mut filters: HashMap<String, Vec<String>> = HashMap::new();
        let dangling = if all { "false" } else { "true" };
        filters.insert("dangling".to_string(), vec![dangling.to_string()]);
        let opts = bollard::image::PruneImagesOptions { filters };
        let r = d.prune_images(Some(opts)).await.map_err(map_err)?;
        Ok(PruneOutcome {
            deleted: r
                .images_deleted
                .unwrap_or_default()
                .into_iter()
                .filter_map(|i| i.deleted.or(i.untagged))
                .collect(),
            space_reclaimed_bytes: r.space_reclaimed.unwrap_or(0) as u64,
        })
    }

    async fn prune_volumes(&self) -> Result<PruneOutcome, DomainError> {
        let d = docker()?;
        let r = d
            .prune_volumes(None::<bollard::volume::PruneVolumesOptions<String>>)
            .await
            .map_err(map_err)?;
        Ok(PruneOutcome {
            deleted: r.volumes_deleted.unwrap_or_default(),
            space_reclaimed_bytes: r.space_reclaimed.unwrap_or(0) as u64,
        })
    }

    async fn prune_networks(&self) -> Result<PruneOutcome, DomainError> {
        let d = docker()?;
        let r = d
            .prune_networks(None::<bollard::network::PruneNetworksOptions<String>>)
            .await
            .map_err(map_err)?;
        Ok(PruneOutcome {
            deleted: r.networks_deleted.unwrap_or_default(),
            space_reclaimed_bytes: 0,
        })
    }

    async fn prune_build_cache(&self, all: bool) -> Result<PruneOutcome, DomainError> {
        let resp = prune_build_cache_call(all).await?;
        Ok(PruneOutcome {
            deleted: resp.caches_deleted.unwrap_or_default(),
            space_reclaimed_bytes: resp.space_reclaimed.unwrap_or(0) as u64,
        })
    }
}

/// Appel bas niveau de `POST /build/prune` sur le socket Docker (absent de bollard
/// 0.18). Ouvre une connexion HTTP/1 sur `/var/run/docker.sock` via hyper.
#[cfg(unix)]
async fn prune_build_cache_call(
    all: bool,
) -> Result<bollard::models::BuildPruneResponse, DomainError> {
    use http_body_util::BodyExt;
    use http_body_util::Empty;
    use hyper::body::Bytes;

    let err = DomainError::Internal;

    let stream = tokio::net::UnixStream::connect("/var/run/docker.sock")
        .await
        .map_err(|e| err(format!("docker socket: {e}")))?;
    let io = hyper_util::rt::TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .map_err(|e| err(format!("docker http handshake: {e}")))?;
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let req = hyper::Request::builder()
        .method("POST")
        .uri(format!("/build/prune?all={all}"))
        .header(hyper::header::HOST, "localhost")
        .body(Empty::<Bytes>::new())
        .map_err(|e| err(format!("docker request: {e}")))?;

    let res = sender
        .send_request(req)
        .await
        .map_err(|e| err(format!("docker /build/prune: {e}")))?;
    let status = res.status();
    let body = res
        .into_body()
        .collect()
        .await
        .map_err(|e| err(format!("docker /build/prune body: {e}")))?
        .to_bytes();

    if !status.is_success() {
        return Err(err(format!(
            "docker /build/prune HTTP {}: {}",
            status.as_u16(),
            String::from_utf8_lossy(&body)
        )));
    }
    serde_json::from_slice(&body).map_err(|e| err(format!("docker /build/prune decode: {e}")))
}

#[cfg(not(unix))]
async fn prune_build_cache_call(
    _all: bool,
) -> Result<bollard::models::BuildPruneResponse, DomainError> {
    Err(DomainError::Internal(
        "purge du build cache indisponible : socket Docker unix requis".into(),
    ))
}
