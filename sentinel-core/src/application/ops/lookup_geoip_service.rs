//! Implementation du use case GeoIP. Pass-through vers le port outbound
//! (l'appel HTTP au fournisseur est dans l'adapter).

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::ops::geoip::GeoIpEntry;
use crate::domain::errors::DomainError;
use crate::ports::inbound::ops::lookup_geoip::LookupGeoIpUseCase;
use crate::ports::outbound::ops::geoip_lookup::GeoIpLookup;

pub struct LookupGeoIpService {
    lookup: Arc<dyn GeoIpLookup>,
}

impl LookupGeoIpService {
    pub fn new(lookup: Arc<dyn GeoIpLookup>) -> Self {
        Self { lookup }
    }
}

#[async_trait]
impl LookupGeoIpUseCase for LookupGeoIpService {
    async fn lookup(&self, ips: Vec<String>) -> Result<Vec<GeoIpEntry>, DomainError> {
        self.lookup.lookup(ips).await
    }
}
