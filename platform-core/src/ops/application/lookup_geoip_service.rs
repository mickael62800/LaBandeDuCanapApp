//! Implementation du use case GeoIP. Pass-through vers le port outbound
//! (l'appel HTTP au fournisseur est dans l'adapter).

use std::sync::Arc;

use async_trait::async_trait;

use crate::ops::domain::entities::geoip::GeoIpEntry;
use crate::ops::domain::errors::DomainError;
use crate::ops::ports::inbound::lookup_geoip::LookupGeoIpUseCase;
use crate::ops::ports::outbound::geoip_lookup::GeoIpLookup;

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
