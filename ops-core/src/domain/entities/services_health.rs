//! Sante des services de l'installation : combien de bots et de workers
//! repondent, et l'etat du backend de decouverte (Redis).
//!
//! Vivait dans `DashboardStats` cote Sentinel, ce qui melangeait le metier
//! Discord et l'exploitation de la machine — et forcait `sentinel-core` a
//! dependre d'`ops-core`. Ces chiffres decrivent l'installation, pas le
//! serveur Discord : ils appartiennent a l'exploitation.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ServicesHealth {
    pub bots_online: u32,
    pub bots_total: u32,
    pub workers_online: u32,
    pub workers_total: u32,
    /// Backend de decouverte joignable. Faux = les compteurs ci-dessus valent
    /// zero faute d'information, pas parce que tout est arrete.
    pub redis_online: bool,
}
