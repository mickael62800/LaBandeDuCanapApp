//! Use case : resolution GeoIP d'une liste d'IPs (panel securite).

use async_trait::async_trait;

use crate::ops::domain::entities::geoip::GeoIpEntry;
use crate::ops::domain::errors::DomainError;

#[async_trait]
pub trait LookupGeoIpUseCase: Send + Sync {
    async fn lookup(&self, ips: Vec<String>) -> Result<Vec<GeoIpEntry>, DomainError>;
}
