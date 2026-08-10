//! Statistiques du tableau de bord — partie METIER uniquement.
//!
//! Cette entite portait aussi la sante de l'infrastructure (bots et workers en
//! ligne, Redis) : elle melangeait donc le metier Discord et l'exploitation de
//! la machine. C'est ce qui obligeait `ManageStatsService` a consommer le port
//! `ServiceRegistry` d'`ops-core`, et donc `sentinel-core` a dependre de
//! l'exploitation.
//!
//! Ces champs vivent desormais dans `ops_core::domain::entities::services_health`.
//! C'est l'adaptateur HTTP qui compose les deux pour le tableau de bord : reunir
//! des donnees de deux domaines est le travail d'un adaptateur, pas celui d'un
//! service applicatif.
//!
//! `postgres_online` reste ici : c'est la disponibilite de la base de Sentinel,
//! constatee en lisant ses propres tables.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_servers: u32,
    pub total_users: u32,
    pub messages_today: u64,
    pub infractions_today: u32,
    pub postgres_online: bool,
}
