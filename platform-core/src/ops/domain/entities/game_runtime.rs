//! Entités de domaine pour le cycle de vie des conteneurs applicatifs pilotés
//! sur l'hôte (aujourd'hui : les serveurs de jeu de Nexus).
//!
//! # Pourquoi ici et pas dans `nexus-core`
//!
//! Ces types décrivent ce qu'on demande au daemon Docker de l'hôte, pas les
//! règles du portail de jeux. Ils vivaient dans `nexus-core`, ce qui obligeait
//! `nexus-api` à porter `bollard` et à monter `/var/run/docker.sock` — soit un
//! équivalent root sur l'hôte dans le processus qui sert aussi les routes du
//! portail. En les remontant ici, `docker-agent` (le seul processus autorisé à
//! voir le socket) peut les implémenter, et `nexus-api` redevient un simple
//! client HTTP.
//!
//! `nexus-core` les ré-exporte : ses use cases et ses tests sont inchangés.
//!
//! Aucune dépendance bollard ici — même règle que `docker_host`. Les types
//! portent `Serialize`/`Deserialize` parce qu'ils traversent le réseau entre
//! `nexus-api` et `docker-agent`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Specification d'un container a creer. Genere par le use case a partir
/// du template + config + ports alloues. Le runtime fait juste l'execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    /// Image Docker (whitelist verifiee par le caller).
    pub image: String,
    /// Nom du container Docker (unique).
    pub name: String,
    /// Variables d'environnement complete'es (defaults + overrides).
    pub env: HashMap<String, String>,
    /// Port mappings host_port -> container_port.
    pub port_mappings: Vec<PortMapping>,
    /// Volume nomme a monter sur un point de montage interne.
    /// Format : (volume_name, container_path).
    pub volumes: Vec<VolumeMount>,
    /// Memoire max (bytes). Hard-limit Docker.
    pub memory_bytes: u64,
    /// Plafond CPU en nombre de coeurs (2.0 = deux coeurs pleins). None =
    /// plafond par defaut de l'adapter.
    ///
    /// C'est une PROTECTION, pas un accelerateur : un serveur ne va pas plus
    /// vite parce qu'on lui donne des coeurs, mais un serveur emballe ne peut
    /// plus asphyxier les autres ni la base de donnees.
    pub cpu_limit: Option<f64>,
    /// Network name (cree au boot si absent).
    pub network: String,
    /// User non-root applique (--user UID:GID). None = laisse le default de l'image.
    pub user: Option<String>,
    /// Restart policy : "no" (par defaut), "on-failure:N".
    pub restart_policy: RestartPolicy,
    /// Labels Docker pour traçabilite (sentinel.* — lecture par le reconciler).
    pub labels: HashMap<String, String>,
    /// Override de la commande Docker (Cmd). None = laisse l'image decider.
    /// Cas d'usage : Terraria/ryshe ou il faut passer -autocreate, -world,
    /// etc. via flags CLI car l'image ne lit pas tout depuis l'env.
    pub command: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: PortProtocol,
    /// Adresse host sur laquelle binder le port. "0.0.0.0" pour un port
    /// jeu (exposé au reseau), "127.0.0.1" pour un port d'admin (RCON) qui
    /// ne doit etre joignable que depuis l'app locale.
    pub host_ip: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortProtocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub volume_name: String,
    pub container_path: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartPolicy {
    None,
    OnFailure(u32),
}

/// Etat observe d'un container (par inspect).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatus {
    pub container_id: String,
    pub state: ContainerState,
    pub exit_code: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerState {
    Created,
    Running,
    Restarting,
    Paused,
    Exited,
    Dead,
}

/// Stats temps-reel d'un container.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_limit_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

/// Container detecte par le reconciler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedContainer {
    pub container_id: String,
    pub name: String,
    pub state: ContainerState,
    pub labels: HashMap<String, String>,
}

/// Archive d'un volume de jeu, telle que l'agent la rend une fois ecrite.
///
/// Le chemin est celui vu par l'AGENT, pas par l'appelant : c'est lui qui monte
/// le repertoire de sauvegarde, et lui seul sait ou l'archive a atterri. Nexus
/// se contente de le consigner dans `game_backups`, pour que l'on retrouve le
/// fichier plus tard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeArchive {
    pub path: String,
    pub size_bytes: u64,
}
