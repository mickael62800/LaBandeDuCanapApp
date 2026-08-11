//! Implementation `GameContainerRuntime` via bollard (Docker socket).
//!
//! Deplace depuis `nexus-api/src/adapters/outbound/game_runtime/docker_runtime.rs`
//! sans changement de comportement : seuls les imports bougent. Le but du
//! deplacement est que le mapping bollard -> domaine n'existe qu'ICI, dans le
//! seul processus qui voit `/var/run/docker.sock`. `nexus-api` en garde un
//! client HTTP, comme `sentinel-api` avec `HttpDockerHost`.
//!
//! Securite par construction :
//!  - Pas de cmd ni d'entrypoint custom passe au caller (l'image decide).
//!  - Pas de bind-mount host : uniquement des volumes Docker nommes
//!    (le caller passe juste un nom + chemin interne).
//!  - Pas de --privileged, pas de --pid host, pas de --network host :
//!    l'API ne les expose tout simplement pas.
//!  - User non-root par defaut (--user UID:GID), defini par le caller.
//!  - Network specifique configure (sentinel-games), isole.
//!  - Les requetes echouent fail-closed : si le caller ne fournit pas un
//!    network, on n'utilise pas le default bridge -> create_container error.

use async_trait::async_trait;
use bollard::container::{
    Config as BollardConfig, CreateContainerOptions, ListContainersOptions, LogsOptions,
    RemoveContainerOptions, StartContainerOptions, Stats as BollardStats, StatsOptions,
    StopContainerOptions, UploadToContainerOptions,
};
use bollard::image::{CreateImageOptions, ListImagesOptions};
use bollard::models::{
    HostConfig, HostConfigLogConfig, Mount, MountTypeEnum, PortBinding, ResourcesUlimits,
    RestartPolicy as BollardRestartPolicy, RestartPolicyNameEnum,
};
use bollard::network::CreateNetworkOptions;
use bollard::volume::CreateVolumeOptions;
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;

use ops_core::domain::entities::game_runtime::{
    ContainerSpec, ContainerState, ContainerStats, ContainerStatus, ManagedContainer, PortProtocol,
    RestartPolicy,
};
use ops_core::domain::errors::DomainError;
use ops_core::ports::outbound::game_runtime::GameContainerRuntime;

/// Label canonique du portail de jeux. `nexus.*` et plus `sentinel.*` : les
/// serveurs de jeu ont quitte Sentinel au portage, le prefixe historique ne
/// designait plus rien.
const MANAGED_LABEL_KEY: &str = "nexus.managed";
/// Label de la generation precedente, ECRIT ET LU en plus du nouveau.
///
/// La flotte deja en service ne porte que celui-la. Le reconciler retrouve ses
/// conteneurs par label : basculer d'un coup les aurait tous fait passer pour
/// des orphelins — donc arreter et supprimer par le job de reconciliation. On
/// ecrit les deux et on lit les deux ; le jour ou plus aucun conteneur ne porte
/// l'ancien (verifiable par
/// `docker ps -a --filter label=sentinel.managed=game-portal`), cette constante
/// et les deux boucles qui la lisent disparaissent.
const LEGACY_MANAGED_LABEL_KEY: &str = "sentinel.managed";
const MANAGED_LABEL_VALUE: &str = "game-portal";

/// Plafond CPU par defaut : 2 vCPU (en nano-CPUs, unite Docker). Utilise
/// quand le serveur n'en definit pas. Empeche un container de monopoliser
/// l'host.
const DEFAULT_NANO_CPUS: i64 = 2_000_000_000;

/// Convertit un nombre de coeurs en nano-CPUs Docker (1 coeur = 1e9).
fn nano_cpus(cpu_limit: Option<f64>) -> i64 {
    match cpu_limit {
        Some(c) if c > 0.0 => (c * 1_000_000_000.0) as i64,
        _ => DEFAULT_NANO_CPUS,
    }
}
/// Plafond du nombre de processus/threads (anti fork-bomb).
const CONTAINER_PIDS_LIMIT: i64 = 512;
/// Plafond du nombre de file descriptors ouverts (nofile).
const CONTAINER_NOFILE_LIMIT: i64 = 4096;

/// Construit le client Docker (socket par defaut). Singleton via Arc.
pub fn make_docker_client() -> Result<Arc<Docker>, DomainError> {
    let d = Docker::connect_with_local_defaults()
        .map_err(|e| DomainError::Internal(format!("docker socket: {e}")))?;
    Ok(Arc::new(d))
}

pub struct DockerContainerRuntime {
    docker: Arc<Docker>,
}

impl DockerContainerRuntime {
    pub fn new(docker: Arc<Docker>) -> Self {
        Self { docker }
    }
}

fn map_state(s: Option<&str>) -> ContainerState {
    match s.unwrap_or("") {
        "created" => ContainerState::Created,
        "running" => ContainerState::Running,
        "restarting" => ContainerState::Restarting,
        "paused" => ContainerState::Paused,
        "exited" => ContainerState::Exited,
        "dead" => ContainerState::Dead,
        _ => ContainerState::Dead,
    }
}

fn into_bollard_restart(p: RestartPolicy) -> BollardRestartPolicy {
    match p {
        RestartPolicy::None => BollardRestartPolicy {
            name: Some(RestartPolicyNameEnum::NO),
            maximum_retry_count: None,
        },
        RestartPolicy::OnFailure(n) => BollardRestartPolicy {
            name: Some(RestartPolicyNameEnum::ON_FAILURE),
            maximum_retry_count: Some(n as i64),
        },
    }
}

fn proto_str(p: PortProtocol) -> &'static str {
    match p {
        PortProtocol::Tcp => "tcp",
        PortProtocol::Udp => "udp",
    }
}

/// Calcule le pourcentage CPU a partir des stats bollard. Formule officielle :
/// ((cpu_total_delta - precpu_total_delta) / system_cpu_delta) * online_cpus * 100.
fn compute_cpu_percent(s: &BollardStats) -> f64 {
    let cpu_delta =
        s.cpu_stats.cpu_usage.total_usage as f64 - s.precpu_stats.cpu_usage.total_usage as f64;
    let system_delta = s.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
        - s.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
    let online = s.cpu_stats.online_cpus.unwrap_or(1).max(1) as f64;
    if system_delta > 0.0 && cpu_delta > 0.0 {
        (cpu_delta / system_delta) * online * 100.0
    } else {
        0.0
    }
}

#[async_trait]
impl GameContainerRuntime for DockerContainerRuntime {
    async fn ensure_network(&self, name: &str) -> Result<(), DomainError> {
        let existing = self
            .docker
            .list_networks::<String>(None)
            .await
            .map_err(|e| DomainError::Internal(format!("list networks: {e}")))?;
        if existing.iter().any(|n| n.name.as_deref() == Some(name)) {
            return Ok(());
        }
        self.docker
            .create_network(CreateNetworkOptions {
                name: name.to_string(),
                driver: "bridge".to_string(),
                check_duplicate: true,
                internal: false,
                attachable: true,
                ..Default::default()
            })
            .await
            .map_err(|e| DomainError::Internal(format!("create network: {e}")))?;
        Ok(())
    }

    async fn ensure_volume(&self, name: &str) -> Result<(), DomainError> {
        // create_volume idempotent si meme nom (Docker retourne le volume existant).
        let mut labels: HashMap<&str, &str> = HashMap::new();
        labels.insert(MANAGED_LABEL_KEY, MANAGED_LABEL_VALUE);
        labels.insert(LEGACY_MANAGED_LABEL_KEY, MANAGED_LABEL_VALUE);
        self.docker
            .create_volume(CreateVolumeOptions {
                name,
                driver: "local",
                driver_opts: HashMap::new(),
                labels,
            })
            .await
            .map_err(|e| DomainError::Internal(format!("create volume: {e}")))?;
        Ok(())
    }

    async fn pull_image_if_missing(&self, image: &str) -> Result<(), DomainError> {
        // Check si l'image existe deja localement.
        let mut filters = HashMap::new();
        filters.insert("reference".to_string(), vec![image.to_string()]);
        let existing = self
            .docker
            .list_images(Some(ListImagesOptions {
                all: false,
                filters,
                ..Default::default()
            }))
            .await
            .map_err(|e| DomainError::Internal(format!("list images: {e}")))?;
        if !existing.is_empty() {
            return Ok(());
        }
        // Pull. Bollard retourne un stream ; on draine jusqu'a la fin.
        let opts = CreateImageOptions {
            from_image: image.to_string(),
            ..Default::default()
        };
        let mut stream = self.docker.create_image(Some(opts), None, None);
        while let Some(item) = stream.next().await {
            item.map_err(|e| DomainError::Internal(format!("pull image: {e}")))?;
        }
        Ok(())
    }

    async fn create_container(&self, spec: &ContainerSpec) -> Result<String, DomainError> {
        // ── Construction des port bindings ─────────────────────────────
        let mut exposed_ports: HashMap<String, HashMap<(), ()>> = HashMap::new();
        let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();
        for pm in &spec.port_mappings {
            let key = format!("{}/{}", pm.container_port, proto_str(pm.protocol));
            exposed_ports.insert(key.clone(), HashMap::new());
            port_bindings.insert(
                key,
                Some(vec![PortBinding {
                    // Bind IP par mapping : 0.0.0.0 pour les ports jeu,
                    // 127.0.0.1 pour RCON (cf. PortMapping::host_ip).
                    host_ip: Some(pm.host_ip.clone()),
                    host_port: Some(pm.host_port.to_string()),
                }]),
            );
        }

        // ── Mounts (volumes nommes uniquement, JAMAIS bind-mount host) ─
        let mounts: Vec<Mount> = spec
            .volumes
            .iter()
            .map(|v| Mount {
                target: Some(v.container_path.clone()),
                source: Some(v.volume_name.clone()),
                typ: Some(MountTypeEnum::VOLUME),
                read_only: Some(v.read_only),
                ..Default::default()
            })
            .collect();

        // ── Labels (marque le conteneur pour le reconciler) ────────────
        // Les DEUX generations sont posees : un conteneur cree aujourd'hui
        // doit rester visible d'un agent qui n'aurait pas encore ete mis a
        // jour, et inversement.
        let mut labels = spec.labels.clone();
        labels.insert(
            MANAGED_LABEL_KEY.to_string(),
            MANAGED_LABEL_VALUE.to_string(),
        );
        labels.insert(
            LEGACY_MANAGED_LABEL_KEY.to_string(),
            MANAGED_LABEL_VALUE.to_string(),
        );

        // ── Env vars ──────────────────────────────────────────────────
        let env: Vec<String> = spec.env.iter().map(|(k, v)| format!("{k}={v}")).collect();

        let host_config = HostConfig {
            mounts: Some(mounts),
            port_bindings: Some(port_bindings),
            memory: Some(spec.memory_bytes as i64),
            // Hard memory limit : si le container depasse, OOM-killed (proteger l'host).
            memory_swap: Some(spec.memory_bytes as i64),
            // Plafonds CPU / PIDs / fichiers ouverts : protegent l'host
            // contre l'epuisement de ressources par un container.
            nano_cpus: Some(nano_cpus(spec.cpu_limit)),
            pids_limit: Some(CONTAINER_PIDS_LIMIT),
            ulimits: Some(vec![ResourcesUlimits {
                name: Some("nofile".to_string()),
                soft: Some(CONTAINER_NOFILE_LIMIT),
                hard: Some(CONTAINER_NOFILE_LIMIT),
            }]),
            network_mode: Some(spec.network.clone()),
            // Rotation des logs : driver json-file plafonne a 3 fichiers de
            // 10 Mo. Sans ca, un container bavard peut remplir le disque host.
            // (La taille du volume monde reste une limite Docker non couverte
            // ici : pas de quota de volume cote daemon — a gerer hors app.)
            log_config: Some(HostConfigLogConfig {
                typ: Some("json-file".to_string()),
                config: Some(HashMap::from([
                    ("max-size".to_string(), "10m".to_string()),
                    ("max-file".to_string(), "3".to_string()),
                ])),
            }),
            restart_policy: Some(into_bollard_restart(spec.restart_policy)),
            // Securite : pas de privileged, pas de cap_add, pas de pid host.
            privileged: Some(false),
            pid_mode: None,
            ipc_mode: None,
            userns_mode: None,
            // Read-only root filesystem ? Trop strict pour Minecraft (logs, world)
            // -> on garde rw mais avec memoire limitee + user non-root.
            ..Default::default()
        };

        let cfg = BollardConfig {
            image: Some(spec.image.clone()),
            env: Some(env),
            labels: Some(labels),
            exposed_ports: Some(exposed_ports),
            user: spec.user.clone(),
            host_config: Some(host_config),
            // Override CMD si le template le precise (ex : Terraria/ryshe
            // qui exige -autocreate, -world... en flags). None = laisse
            // l'ENTRYPOINT/CMD de l'image inchange.
            cmd: spec.command.clone(),
            ..Default::default()
        };

        let opts = CreateContainerOptions {
            name: spec.name.clone(),
            platform: None,
        };
        let resp = self
            .docker
            .create_container(Some(opts), cfg)
            .await
            .map_err(|e| DomainError::Internal(format!("create container: {e}")))?;
        Ok(resp.id)
    }

    async fn start_container(&self, container_id: &str) -> Result<(), DomainError> {
        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| DomainError::Internal(format!("start container: {e}")))?;
        Ok(())
    }

    async fn upload_file_to_container(
        &self,
        container_id: &str,
        path: &str,
        content: &str,
    ) -> Result<(), DomainError> {
        // bollard attend un tar pose sur un chemin du container. On poste
        // a "/" et on inclut le chemin COMPLET dans l'entry tar (les
        // repertoires intermediaires sont implicitement materialises par
        // tar). Ex : path = "/tshock/config.json" -> entry "tshock/config.json"
        // genere /tshock/config.json (et /tshock si absent).
        let rel_path = path.trim_start_matches('/');
        if rel_path.is_empty() || rel_path.ends_with('/') {
            return Err(DomainError::Internal(format!(
                "upload_file: path invalide '{path}'"
            )));
        }
        // Anti path-traversal : le path est rendu depuis un template avec des
        // env vars controlables par l'utilisateur. On rejette tout segment
        // `..` qui permettrait d'ecrire hors de l'arborescence visee.
        if rel_path.split('/').any(|seg| seg == "..") {
            return Err(DomainError::Internal(format!(
                "upload_file: path traversal interdit '{path}'"
            )));
        }

        // On ajoute aussi des entries directory pour chaque parent, en mode
        // 0755, pour s'assurer que les permissions sont correctes meme si
        // tar standard n'exige pas ces entries.
        let bytes = content.as_bytes();
        let mut buf = Vec::with_capacity(bytes.len() + 2048);
        {
            let mut builder = tar::Builder::new(&mut buf);

            // Entries dir pour chaque segment parent (ex: "tshock/" pour
            // "tshock/config.json"). Idempotent cote tar.
            let parts: Vec<&str> = rel_path.split('/').collect();
            for i in 1..parts.len() {
                let dir = format!("{}/", parts[..i].join("/"));
                let mut h = tar::Header::new_gnu();
                h.set_path(&dir)
                    .map_err(|e| DomainError::Internal(format!("tar dir set_path: {e}")))?;
                h.set_size(0);
                h.set_mode(0o755);
                h.set_entry_type(tar::EntryType::Directory);
                h.set_cksum();
                builder
                    .append(&h, std::io::empty())
                    .map_err(|e| DomainError::Internal(format!("tar dir append: {e}")))?;
            }

            // Entry du fichier lui-meme.
            let mut header = tar::Header::new_gnu();
            header
                .set_path(rel_path)
                .map_err(|e| DomainError::Internal(format!("tar file set_path: {e}")))?;
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append(&header, bytes)
                .map_err(|e| DomainError::Internal(format!("tar file append: {e}")))?;
            builder
                .finish()
                .map_err(|e| DomainError::Internal(format!("tar finish: {e}")))?;
        }

        let opts = UploadToContainerOptions {
            path: "/".to_string(),
            no_overwrite_dir_non_dir: "0".to_string(),
        };
        self.docker
            .upload_to_container(container_id, Some(opts), buf.into())
            .await
            .map_err(|e| DomainError::Internal(format!("upload_to_container: {e}")))?;
        Ok(())
    }

    async fn stop_container(
        &self,
        container_id: &str,
        timeout_secs: u32,
    ) -> Result<(), DomainError> {
        self.docker
            .stop_container(
                container_id,
                Some(StopContainerOptions {
                    t: timeout_secs as i64,
                }),
            )
            .await
            .map_err(|e| DomainError::Internal(format!("stop container: {e}")))?;
        Ok(())
    }

    async fn restart_container(
        &self,
        container_id: &str,
        timeout_secs: u32,
    ) -> Result<(), DomainError> {
        self.docker
            .restart_container(
                container_id,
                Some(bollard::container::RestartContainerOptions {
                    t: timeout_secs as isize,
                }),
            )
            .await
            .map_err(|e| DomainError::Internal(format!("restart container: {e}")))?;
        Ok(())
    }

    async fn remove_container(&self, container_id: &str) -> Result<(), DomainError> {
        self.docker
            .remove_container(
                container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    v: false,
                    link: false,
                }),
            )
            .await
            .map_err(|e| DomainError::Internal(format!("remove container: {e}")))?;
        Ok(())
    }

    async fn remove_volume(&self, name: &str) -> Result<(), DomainError> {
        self.docker
            .remove_volume(name, None)
            .await
            .map_err(|e| DomainError::Internal(format!("remove volume: {e}")))?;
        Ok(())
    }

    async fn remove_image(&self, image: &str, force: bool) -> Result<bool, DomainError> {
        let opts = bollard::image::RemoveImageOptions {
            force,
            noprune: false,
        };
        match self.docker.remove_image(image, Some(opts), None).await {
            Ok(_) => Ok(true),
            Err(e) => {
                let msg = e.to_string();
                // Image absente -> on considere ca comme deja propre, pas une erreur.
                if msg.contains("404") || msg.contains("No such image") {
                    return Ok(false);
                }
                // Image encore utilisee par un container -> on log et retourne false
                // sans crash (cas attendu si delete pas tout-a-fait fini).
                if msg.contains("conflict") || msg.contains("being used") {
                    tracing::warn!(image, "remove_image: image encore utilisee, skip");
                    return Ok(false);
                }
                Err(DomainError::Internal(format!("remove image {image}: {e}")))
            }
        }
    }

    async fn inspect(&self, container_id: &str) -> Result<Option<ContainerStatus>, DomainError> {
        let resp = match self.docker.inspect_container(container_id, None).await {
            Ok(r) => r,
            Err(e) => {
                if e.to_string().contains("404") {
                    return Ok(None);
                }
                return Err(DomainError::Internal(format!("inspect: {e}")));
            }
        };
        let state = resp
            .state
            .as_ref()
            .and_then(|s| s.status.as_ref().map(|st| format!("{st:?}")));
        let exit_code = resp.state.as_ref().and_then(|s| s.exit_code);
        let error = resp
            .state
            .as_ref()
            .and_then(|s| s.error.clone())
            .filter(|s| !s.is_empty());
        Ok(Some(ContainerStatus {
            container_id: resp.id.unwrap_or_default(),
            state: map_state(state.as_deref()),
            exit_code,
            error,
        }))
    }

    async fn stats(&self, container_id: &str) -> Result<ContainerStats, DomainError> {
        let mut stream = self.docker.stats(
            container_id,
            Some(StatsOptions {
                stream: false,
                one_shot: true,
            }),
        );
        let s = stream
            .next()
            .await
            .ok_or_else(|| DomainError::Internal("stats: stream vide".into()))?
            .map_err(|e| DomainError::Internal(format!("stats: {e}")))?;
        let cpu = compute_cpu_percent(&s);
        let mem_used = s.memory_stats.usage.unwrap_or(0);
        let mem_limit = s.memory_stats.limit.unwrap_or(0);
        // Aggregate network rx/tx tous interfaces confondues.
        let (rx, tx) = if let Some(networks) = s.networks {
            networks
                .values()
                .fold((0u64, 0u64), |(r, t), n| (r + n.rx_bytes, t + n.tx_bytes))
        } else {
            (0, 0)
        };
        Ok(ContainerStats {
            cpu_percent: cpu,
            memory_used_bytes: mem_used,
            memory_limit_bytes: mem_limit,
            network_rx_bytes: rx,
            network_tx_bytes: tx,
        })
    }

    async fn logs(&self, container_id: &str, lines: u32) -> Result<Vec<String>, DomainError> {
        let mut stream = self.docker.logs(
            container_id,
            Some(LogsOptions::<String> {
                follow: false,
                stdout: true,
                stderr: true,
                tail: lines.to_string(),
                timestamps: true,
                ..Default::default()
            }),
        );
        let mut out = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| DomainError::Internal(format!("logs: {e}")))?;
            out.push(chunk.to_string());
        }
        Ok(out)
    }

    async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError> {
        // DEUX passes, une par generation de label. Docker combine les filtres
        // `label` en ET, pas en OU : une seule requete portant les deux ne
        // renverrait que les conteneurs qui ont les DEUX — c'est-a-dire aucun
        // de ceux deja en service. On interroge donc separement et on
        // deduplique par identifiant.
        let mut seen: HashMap<String, ManagedContainer> = HashMap::new();

        for key in [MANAGED_LABEL_KEY, LEGACY_MANAGED_LABEL_KEY] {
            let mut filters = HashMap::new();
            filters.insert(
                "label".to_string(),
                vec![format!("{key}={MANAGED_LABEL_VALUE}")],
            );
            let containers = self
                .docker
                .list_containers(Some(ListContainersOptions {
                    all: true,
                    filters,
                    ..Default::default()
                }))
                .await
                .map_err(|e| DomainError::Internal(format!("list managed: {e}")))?;

            for c in containers {
                let container_id = c.id.unwrap_or_default();
                if container_id.is_empty() {
                    continue;
                }
                seen.entry(container_id.clone())
                    .or_insert_with(|| ManagedContainer {
                        container_id,
                        name: c
                            .names
                            .and_then(|n| n.into_iter().next())
                            .unwrap_or_default()
                            .trim_start_matches('/')
                            .to_string(),
                        state: map_state(c.state.as_deref()),
                        labels: c.labels.unwrap_or_default(),
                    });
            }
        }

        Ok(seen.into_values().collect())
    }
}
