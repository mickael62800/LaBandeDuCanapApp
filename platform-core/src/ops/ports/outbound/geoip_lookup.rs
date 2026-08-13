//! Port outbound : resolution GeoIP (fournisseur externe).

use async_trait::async_trait;

use crate::ops::domain::entities::geoip::GeoIpEntry;
use crate::ops::domain::errors::DomainError;

#[async_trait]
pub trait GeoIpLookup: Send + Sync {
    /// Resout une liste d'IPs. Renvoie un vecteur vide si `ips` est vide.
    async fn lookup(&self, ips: Vec<String>) -> Result<Vec<GeoIpEntry>, DomainError>;
}
