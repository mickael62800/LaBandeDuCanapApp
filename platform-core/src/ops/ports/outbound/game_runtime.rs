//! Port outbound : cycle de vie des conteneurs applicatifs sur l'hôte.
//!
//! Deux implémentations existent, et la distinction est le cœur du sujet :
//!
//! - `docker-agent/src/bollard_game.rs` — la vraie, avec `bollard` et le
//!   socket. C'est le SEUL processus du dépôt qui voit `/var/run/docker.sock`.
//! - `nexus-api/src/adapters/outbound/game_runtime/http_runtime.rs` — un client
//!   HTTP vers l'agent. `nexus-api` ignore que Docker est de l'autre côté d'un
//!   appel réseau, exactement comme `sentinel-api` avec `HttpDockerHost`.
//!
//! La liste ci-dessous est une **liste blanche**, pas une API Docker générique.
//! C'est ce qui distingue l'agent d'un passe-plat : qui l'appelle ne peut faire
//! que ces seize choses.

use async_trait::async_trait;

use crate::ops::domain::entities::game_runtime::{
    ContainerSpec, ContainerStats, ContainerStatus, ManagedContainer, VolumeArchive,
};
use crate::ops::domain::errors::DomainError;

#[async_trait]
pub trait GameContainerRuntime: Send + Sync {
    /// Ce runtime peut-il reellement piloter des conteneurs ?
    ///
    /// `false` pour l'implementation de repli, utilisee quand le socket
    /// Docker est indisponible. Elle laisse le listing et la configuration
    /// fonctionner mais echoue sur toute operation de cycle de vie.
    ///
    /// Permet de REFUSER une creation d'emblee au lieu de laisser fabriquer
    /// un serveur qui ne demarrera jamais. Par defaut `true` : un runtime qui
    /// ne se declare pas est suppose fonctionnel.
    fn is_operational(&self) -> bool {
        true
    }

    /// Cree le network dedie (idempotent).
    async fn ensure_network(&self, name: &str) -> Result<(), DomainError>;

    /// Cree le volume nomme (idempotent).
    async fn ensure_volume(&self, name: &str) -> Result<(), DomainError>;

    /// Pull l'image si absente. Bloquant.
    async fn pull_image_if_missing(&self, image: &str) -> Result<(), DomainError>;

    /// Cree le container. Retourne son id Docker. Ne le demarre PAS.
    async fn create_container(&self, spec: &ContainerSpec) -> Result<String, DomainError>;

    /// Demarre un container existant.
    async fn start_container(&self, container_id: &str) -> Result<(), DomainError>;

    /// Pose un fichier (utf-8) sur le filesystem du container, a un chemin
    /// absolu. A appeler entre `create_container` et `start_container` —
    /// les volumes nommes sont deja montes a ce stade. Les sous-repertoires
    /// inexistants sont crees. Permet de seed des fichiers de config que
    /// l'image ne genere pas elle-meme (ex : ryshe/terraria + config.json).
    async fn upload_file_to_container(
        &self,
        container_id: &str,
        path: &str,
        content: &str,
    ) -> Result<(), DomainError>;

    /// Arrete proprement (SIGTERM puis SIGKILL apres `timeout_secs`).
    async fn stop_container(
        &self,
        container_id: &str,
        timeout_secs: u32,
    ) -> Result<(), DomainError>;

    /// Restart (= stop + start, geres en interne).
    async fn restart_container(
        &self,
        container_id: &str,
        timeout_secs: u32,
    ) -> Result<(), DomainError>;

    /// Supprime le container (force).
    async fn remove_container(&self, container_id: &str) -> Result<(), DomainError>;

    /// Supprime un volume nomme (ne casse rien si en cours d'utilisation,
    /// retourne une erreur dans ce cas).
    async fn remove_volume(&self, name: &str) -> Result<(), DomainError>;

    /// Archive le contenu d'un volume de jeu vers le repertoire de sauvegarde
    /// de l'agent, sous le nom de fichier demande.
    ///
    /// A n'appeler que conteneur ARRETE : une archive prise pendant que le jeu
    /// ecrit peut contenir un fichier a moitie sauvegarde, ce qui ne se voit
    /// qu'au moment de restaurer. Le seul moment ou cette condition est
    /// naturellement remplie est le redemarrage programme, entre l'arret et la
    /// relance — c'est de la que l'appel est fait.
    ///
    /// `nom_fichier` ne doit designer qu'un nom, sans separateur de chemin :
    /// l'agent choisit le repertoire, l'appelant ne peut pas ecrire ailleurs.
    async fn archive_volume(
        &self,
        volume: &str,
        nom_fichier: &str,
    ) -> Result<VolumeArchive, DomainError>;

    /// Supprime les archives d'un serveur, sauf les `garder` plus recentes.
    ///
    /// APPELEE A LA SUPPRESSION D'UN SERVEUR. Le volume part avec lui, mais ses
    /// archives lui survivent sur le disque — plusieurs gigaoctets par monde,
    /// pour un serveur qui n'existe plus. On les efface toutes sauf la
    /// derniere : garder une trace du monde tel qu'il etait le dernier soir
    /// coute peu de place, et c'est la seule chose qu'on regrettera d'avoir
    /// perdue.
    ///
    /// Defaut inoffensif : une implementation qui ne gere pas les archives ne
    /// doit pas faire echouer une suppression de serveur.
    ///
    /// Rend le nombre d'archives supprimees et l'espace libere.
    async fn prune_archives(
        &self,
        _prefixe: &str,
        _garder: usize,
    ) -> Result<(usize, u64), DomainError> {
        Ok((0, 0))
    }

    /// Supprime une image Docker. Retourne true si supprimee, false si
    /// l'image n'existait pas / etait encore utilisee. force=true tente
    /// la suppression meme si des containers stoppes l'utilisent.
    async fn remove_image(&self, image: &str, force: bool) -> Result<bool, DomainError>;

    /// Inspect : retourne le status courant.
    async fn inspect(&self, container_id: &str) -> Result<Option<ContainerStatus>, DomainError>;

    /// Stats (snapshot one-shot, pas un stream).
    async fn stats(&self, container_id: &str) -> Result<ContainerStats, DomainError>;

    /// Logs (last N lines), pas de follow.
    async fn logs(&self, container_id: &str, lines: u32) -> Result<Vec<String>, DomainError>;

    /// Liste tous les conteneurs pilotes par le portail de jeux, pour le
    /// reconciler. L'implementation filtre sur `nexus.managed=game-portal` ET
    /// sur le label historique `sentinel.managed=game-portal` : la flotte creee
    /// avant le renommage ne porte que le second, et l'omettre la ferait passer
    /// pour orpheline.
    async fn list_managed_containers(&self) -> Result<Vec<ManagedContainer>, DomainError>;
}
