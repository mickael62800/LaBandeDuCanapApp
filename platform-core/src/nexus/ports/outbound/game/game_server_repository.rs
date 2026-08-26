use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::nexus::domain::entities::game::server::{GameServer, GameServerStatus};
use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait GameServerRepository: Send + Sync {
    /// Insere une ligne en statut `created`. Retourne l'entite avec id genere.
    async fn create(&self, server: NewGameServer) -> Result<GameServer, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<GameServer>, DomainError>;

    /// Retourne en lot les identifiants qui correspondent encore a un serveur
    /// non supprime. L'implementation par defaut garde les adapters de test
    /// simples ; les adapters de stockage doivent surcharger pour effectuer
    /// une seule requete.
    async fn find_existing_ids(&self, ids: &[Uuid]) -> Result<HashSet<Uuid>, DomainError> {
        let mut existing = HashSet::with_capacity(ids.len());
        for id in ids {
            if self.find_by_id(*id).await?.is_some() {
                existing.insert(*id);
            }
        }
        Ok(existing)
    }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<GameServer>, DomainError>;
    async fn list_running(&self) -> Result<Vec<GameServer>, DomainError>;
    async fn list_active(&self) -> Result<Vec<GameServer>, DomainError>;

    /// Maj champs critiques (status, container_id, ports, volume) — atomique.
    async fn update_runtime(
        &self,
        id: Uuid,
        update: GameServerRuntimeUpdate,
    ) -> Result<(), DomainError>;

    async fn update_status(
        &self,
        id: Uuid,
        status: GameServerStatus,
        last_error: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Transition de statut ATOMIQUE conditionnelle. Passe le serveur de
    /// l'un des etats `from` vers `to` en une seule requete
    /// (`UPDATE ... WHERE id = $1 AND status = ANY(from)`). Retourne `true`
    /// si la ligne a bien ete mise a jour (claim reussi), `false` si le
    /// statut courant n'etait dans aucun des `from` (quelqu'un d'autre a
    /// deja pris la transition / etat incompatible). Sert de verrou contre
    /// les start/stop concurrents.
    async fn try_transition_status(
        &self,
        id: Uuid,
        from: &[GameServerStatus],
        to: GameServerStatus,
    ) -> Result<bool, DomainError>;

    async fn update_player_activity(&self, id: Uuid, player_count: i32) -> Result<(), DomainError>;

    /// Change les ressources allouees a un serveur.
    ///
    /// Docker fige memoire et processeur a la CREATION du conteneur : ces
    /// valeurs ne prendront effet qu'a sa reconstruction, comme la
    /// configuration (cf. `config_dirty`).
    async fn update_resources(
        &self,
        id: Uuid,
        memory_mb: i32,
        cpu_limit: Option<f64>,
    ) -> Result<(), DomainError>;

    /// Enregistre une tentative de redemarrage auto : incremente
    /// `restart_attempts` et pose `last_restart_at = NOW()`. Sert au backoff.
    async fn record_restart_attempt(&self, id: Uuid) -> Result<(), DomainError>;

    /// Remet `restart_attempts` a 0 (serveur recupere). No-op si deja a 0.
    async fn reset_restart_attempts(&self, id: Uuid) -> Result<(), DomainError>;

    /// Soft-delete (status = deleted, deleted_at = NOW()).
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Compte les serveurs actifs (non-deleted) d'une guild + leur memoire totale.
    /// Pour le calcul de quota.
    async fn count_active_for_guild(&self, guild_id: &str) -> Result<(i32, i32), DomainError>;

    /// Pour les templates demandes, retourne (nb_servers_actifs,
    /// derniere_activite) en lot.
    /// derniere_activite = MAX(updated_at) sur tous les serveurs (incluant
    /// soft-deleted) qui ont utilise ce template. Utilise par le job
    /// image-cleanup pour decider si l'image Docker peut etre supprimee.
    async fn template_usages(
        &self,
        template_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, TemplateUsage>, DomainError>;

    /// Enregistre les salons Discord (texte + vocal) crees pour la session.
    /// Pose/efface les salons de session. Renvoie `true` si l'ecriture a bien
    /// eu lieu ; quand on POSE des salons (valeur non nulle) c'est un claim garde
    /// qui echoue (`false`) si des salons sont deja enregistres (anti-doublon).
    async fn set_session_channels(
        &self,
        id: Uuid,
        text_channel_id: Option<&str>,
        voice_channel_id: Option<&str>,
    ) -> Result<bool, DomainError>;

    /// Enregistre les noms libres des salons. Les trois sont ecrits ensemble :
    /// `None` signifie « pas de nom libre », et doit donc effacer la valeur
    /// precedente plutot que la laisser en place.
    async fn set_channel_names(
        &self,
        id: uuid::Uuid,
        registration: Option<&str>,
        private: Option<&str>,
        voice: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Compte une tentative de redaction d'annonce.
    ///
    /// Appelee AVANT l'appel au redacteur, pas apres : comptee apres, une panne
    /// entre l'appel et l'ecriture ne laisserait aucune trace et le plafond de
    /// reprise ne serait jamais atteint.
    async fn compter_tentative_annonce(&self, id: uuid::Uuid) -> Result<(), DomainError>;

    /// L'annonce a ete publiee : la session ne doit plus etre reprise.
    async fn marquer_annonce_publiee(&self, id: uuid::Uuid) -> Result<(), DomainError>;

    /// Sessions dont l'annonce reste a publier.
    ///
    /// Un salon existe (donc la session est ouverte), aucune annonce n'a ete
    /// publiee, et le plafond de tentatives n'est pas atteint.
    async fn annonces_en_attente(
        &self,
        tentatives_max: i32,
    ) -> Result<Vec<GameServer>, DomainError>;

    /// Marque l'IP comme revelee (le job de revelation l'a publiee).
    async fn mark_ip_revealed(&self, id: Uuid) -> Result<(), DomainError>;

    /// Sessions dont l'IP doit etre revelee maintenant (non revelee,
    /// `ip_reveal_at <= now`, salon cree, non supprimee).
    async fn list_ip_reveal_due(&self) -> Result<Vec<GameServer>, DomainError>;

    /// Sessions en attente de revelation (IP encore masquee, salon cree) et
    /// qui n'ont pas encore recu leur ping du jour. Pour le ping quotidien.
    async fn list_awaiting_reveal_no_ping_today(&self) -> Result<Vec<GameServer>, DomainError>;

    /// Marque qu'un ping quotidien vient d'etre emis pour cette session.
    async fn mark_daily_ping(&self, id: Uuid) -> Result<(), DomainError>;

    /// Definit la date de revelation de l'IP (None pour desactiver).
    /// Enregistre les mesures de reactivite relevees par le controle de sante.
    ///
    /// Les compteurs reseau sont ceux de MAINTENANT : c'est l'appel suivant
    /// qui en tirera un debit, par difference.
    async fn record_perf_sample(
        &self,
        id: uuid::Uuid,
        rcon_latency_ms: Option<i32>,
        net_rx_bytes: Option<i64>,
        net_tx_bytes: Option<i64>,
    ) -> Result<(), DomainError>;

    /// Ajoute un point a l'historique de surveillance.
    ///
    /// Distinct de `record_perf_sample`, qui ECRASE le dernier releve sur la
    /// fiche du serveur : celui-ci conserve. L'un sert a afficher l'instant,
    /// l'autre a regarder une journee.
    ///
    /// Chaque mesure est optionnelle et le reste : une console illisible ne
    /// vaut pas une latence nulle, ni un serveur vide. Ecrire zero a la place
    /// dessinerait une courbe qui ment.
    #[allow(clippy::too_many_arguments)]
    async fn record_history(
        &self,
        id: uuid::Uuid,
        cpu_percent: Option<f32>,
        memory_used_mb: Option<i32>,
        memory_limit_mb: Option<i32>,
        rcon_latency_ms: Option<i32>,
        net_rx_bytes_per_sec: Option<i64>,
        net_tx_bytes_per_sec: Option<i64>,
        player_count: Option<i32>,
    ) -> Result<(), DomainError>;

    /// Historique de surveillance, resume en tranches de `pas_secondes`.
    ///
    /// L'agregation se fait dans la base, pas dans le navigateur : une journee
    /// echantillonnee toutes les trente secondes represente 2 880 points, dont
    /// on ne peut rien lire sur un graphique large de quatre cents pixels.
    ///
    /// Les mesures ne se resument pas toutes de la meme facon : le processeur
    /// et la memoire par leur MOYENNE, la latence par son PIC. C'est le pic qui
    /// fait rager les joueurs, et une moyenne le noierait dans le calme
    /// ambiant.
    async fn history(
        &self,
        id: uuid::Uuid,
        depuis_secondes: i64,
        pas_secondes: i64,
    ) -> Result<Vec<crate::nexus::domain::entities::game::server::PointDeSurveillance>, DomainError>;

    /// Supprime les points de surveillance plus vieux que `jours`.
    ///
    /// Retourne le nombre de lignes effacees. Sans purge, une table de series
    /// temporelles grossit indefiniment — 2 880 lignes par jour et par serveur
    /// en ligne.
    async fn purge_history(&self, jours: i32) -> Result<u64, DomainError>;

    /// Marque (ou lave) l'ecart entre la configuration et le conteneur.
    async fn set_config_dirty(&self, id: uuid::Uuid, dirty: bool) -> Result<(), DomainError>;

    /// Fixe (ou efface) l'heure de fin annoncee de la session.
    async fn set_closes_at(
        &self,
        id: uuid::Uuid,
        at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), DomainError>;

    async fn set_ip_reveal_at(
        &self,
        id: Uuid,
        at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), DomainError>;

    /// Serveurs programmes (`scheduled`) dont le conteneur doit demarrer
    /// maintenant : statut `scheduled`, non supprimes, `ip_reveal_at` non nul
    /// et a moins de `PREP_LEAD_MINUTES` de maintenant. Pour le job auto-start.
    async fn list_scheduled_due_to_start(&self) -> Result<Vec<GameServer>, DomainError>;
}

#[derive(Debug, Clone)]
pub struct TemplateUsage {
    pub active_count: i32,
    pub last_activity_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Donnees pour creer un nouveau serveur.
#[derive(Debug, Clone)]
pub struct NewGameServer {
    pub guild_id: String,
    pub template_id: Uuid,
    pub name: String,
    pub allocated_memory_mb: i32,
    /// Plafond CPU en coeurs. None = defaut de l'adapter.
    pub cpu_limit: Option<f64>,
    pub owner_user_id: String,
    pub idle_shutdown_days: Option<i32>,
    pub initial_config: std::collections::HashMap<String, String>,
}

/// Maj des champs runtime (apres allocation Docker).
#[derive(Debug, Clone, Default)]
pub struct GameServerRuntimeUpdate {
    pub status: Option<GameServerStatus>,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub host_port: Option<u16>,
    pub rcon_port: Option<u16>,
    pub rcon_password: Option<String>,
    pub volume_name: Option<String>,
    pub started_at_now: bool,
    pub stopped_at_now: bool,
    pub last_error: Option<String>,
    pub clear_last_error: bool,
}
