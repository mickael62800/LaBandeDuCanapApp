//! Entités de domaine pour l'administration du daemon Docker de l'hôte.
//!
//! Types simples (aucune dépendance bollard) consommés par le port outbound
//! `DockerHost` et par les handlers HTTP. La règle métier « espace
//! récupérable » (`compute_overview`) est une fonction pure testée ici.

use serde::Deserialize;
use serde::Serialize;

use std::collections::HashMap;

/// Version + compteurs globaux du daemon Docker (merge `version` + `info`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DockerVersionInfo {
    pub version: String,
    pub api_version: String,
    pub os: String,
    pub arch: String,
    pub kernel: String,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
    pub images_count: i64,
}

/// Usage disque d'une image (extrait de `docker system df`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageDiskUsage {
    pub size: i64,
    /// Nombre de containers utilisant cette image (0 = récupérable).
    pub containers: i64,
}

/// Usage disque d'un container (extrait de `docker system df`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerDiskUsage {
    pub size_rw: i64,
    /// État brut du daemon (`"running"`, `"exited"`, ...). `None` si absent.
    pub state: Option<String>,
}

/// Usage disque d'un volume. `size`/`ref_count` sont `None` quand le daemon
/// n'a pas fourni de `usage_data` pour ce volume.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeDiskUsage {
    pub size: Option<i64>,
    pub ref_count: Option<i64>,
}

/// Entrée du build cache (buildkit).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildCacheEntry {
    pub size: i64,
    pub in_use: bool,
}

/// Snapshot `docker system df` complet, en types de domaine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskUsage {
    /// `layers_size` brut du daemon (peut être 0 selon la version de l'API).
    pub layers_size: i64,
    pub images: Vec<ImageDiskUsage>,
    pub containers: Vec<ContainerDiskUsage>,
    pub volumes: Vec<VolumeDiskUsage>,
    pub build_cache: Vec<BuildCacheEntry>,
}

/// Agrégat calculé par [`compute_overview`] : tailles totales et espace
/// récupérable par catégorie.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockerOverview {
    pub volumes_count: i64,
    pub layers_size_bytes: i64,
    pub images_size_bytes: i64,
    pub containers_size_bytes: i64,
    pub volumes_size_bytes: i64,
    pub build_cache_size_bytes: i64,
    pub reclaimable_images_bytes: i64,
    pub reclaimable_containers_bytes: i64,
    pub reclaimable_volumes_bytes: i64,
    pub reclaimable_build_cache_bytes: i64,
}

/// Règle métier « espace récupérable » (fonction pure) :
/// - image sans container (`containers == 0`) → récupérable ;
/// - container non-running (`state != "running"`) → son `size_rw` est récupérable ;
/// - volume avec `ref_count == 0` → récupérable ;
/// - entrée de build cache `!in_use` → récupérable ;
/// - fallback : si `layers_size == 0` (API ne le fournit pas), on retombe sur
///   la somme des tailles d'images.
pub fn compute_overview(usage: &DiskUsage) -> DockerOverview {
    let mut out = DockerOverview {
        volumes_count: usage.volumes.len() as i64,
        layers_size_bytes: usage.layers_size,
        ..Default::default()
    };

    for img in &usage.images {
        out.images_size_bytes += img.size;
        if img.containers == 0 {
            out.reclaimable_images_bytes += img.size;
        }
    }
    if out.layers_size_bytes == 0 {
        out.layers_size_bytes = out.images_size_bytes;
    }

    for c in &usage.containers {
        out.containers_size_bytes += c.size_rw;
        if c.state.as_deref() != Some("running") {
            out.reclaimable_containers_bytes += c.size_rw;
        }
    }

    for v in &usage.volumes {
        if let Some(size) = v.size {
            out.volumes_size_bytes += size;
            if v.ref_count == Some(0) {
                out.reclaimable_volumes_bytes += size;
            }
        }
    }

    for b in &usage.build_cache {
        out.build_cache_size_bytes += b.size;
        if !b.in_use {
            out.reclaimable_build_cache_bytes += b.size;
        }
    }

    out
}

/// Port exposé par un container (mapping brut, formaté côté DTO).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerPort {
    pub private_port: i64,
    pub public_port: Option<i64>,
    /// Protocole en minuscules (`"tcp"`, `"udp"`, ...).
    pub protocol: String,
}

/// Résumé d'un container (listing).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerSummary {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub state: String,
    pub status: String,
    pub created: i64,
    pub size_rw: Option<i64>,
    pub size_root_fs: Option<i64>,
    pub ports: Vec<ContainerPort>,
    pub labels: HashMap<String, String>,
}

/// Résumé d'une image (listing).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageSummary {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub repo_digests: Vec<String>,
    pub created: i64,
    pub size: i64,
    pub shared_size: i64,
    pub virtual_size: i64,
    pub containers: i64,
}

/// Résumé d'un volume (listing).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeSummary {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: Option<String>,
    pub size: Option<i64>,
    pub ref_count: Option<i64>,
}

/// Résumé d'un réseau (listing).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkSummary {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: bool,
    pub containers_count: usize,
}

/// Résultat d'une opération de prune.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PruneOutcome {
    pub deleted: Vec<String>,
    pub space_reclaimed_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_sans_container_est_reclaimable() {
        let usage = DiskUsage {
            layers_size: 500,
            images: vec![
                ImageDiskUsage {
                    size: 100,
                    containers: 0,
                },
                ImageDiskUsage {
                    size: 200,
                    containers: 2,
                },
            ],
            ..Default::default()
        };
        let o = compute_overview(&usage);
        assert_eq!(o.images_size_bytes, 300);
        assert_eq!(o.reclaimable_images_bytes, 100);
        assert_eq!(o.layers_size_bytes, 500);
    }

    #[test]
    fn container_non_running_est_reclaimable() {
        let usage = DiskUsage {
            containers: vec![
                ContainerDiskUsage {
                    size_rw: 10,
                    state: Some("running".into()),
                },
                ContainerDiskUsage {
                    size_rw: 20,
                    state: Some("exited".into()),
                },
                ContainerDiskUsage {
                    size_rw: 5,
                    state: None,
                },
            ],
            ..Default::default()
        };
        let o = compute_overview(&usage);
        assert_eq!(o.containers_size_bytes, 35);
        assert_eq!(o.reclaimable_containers_bytes, 25);
    }

    #[test]
    fn volume_ref_count_zero_est_reclaimable() {
        let usage = DiskUsage {
            volumes: vec![
                VolumeDiskUsage {
                    size: Some(100),
                    ref_count: Some(0),
                },
                VolumeDiskUsage {
                    size: Some(50),
                    ref_count: Some(3),
                },
                // Sans usage_data : ni compté ni récupérable, mais compte
                // dans volumes_count.
                VolumeDiskUsage {
                    size: None,
                    ref_count: None,
                },
            ],
            ..Default::default()
        };
        let o = compute_overview(&usage);
        assert_eq!(o.volumes_count, 3);
        assert_eq!(o.volumes_size_bytes, 150);
        assert_eq!(o.reclaimable_volumes_bytes, 100);
    }

    #[test]
    fn build_cache_hors_usage_est_reclaimable() {
        let usage = DiskUsage {
            build_cache: vec![
                BuildCacheEntry {
                    size: 40,
                    in_use: true,
                },
                BuildCacheEntry {
                    size: 60,
                    in_use: false,
                },
            ],
            ..Default::default()
        };
        let o = compute_overview(&usage);
        assert_eq!(o.build_cache_size_bytes, 100);
        assert_eq!(o.reclaimable_build_cache_bytes, 60);
    }

    #[test]
    fn layers_size_zero_retombe_sur_images_size() {
        let usage = DiskUsage {
            layers_size: 0,
            images: vec![ImageDiskUsage {
                size: 300,
                containers: 1,
            }],
            ..Default::default()
        };
        let o = compute_overview(&usage);
        assert_eq!(o.layers_size_bytes, 300);
    }
}
