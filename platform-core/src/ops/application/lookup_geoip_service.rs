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

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeGeoIpLookup;
    #[async_trait]
    impl GeoIpLookup for FakeGeoIpLookup {
        async fn lookup(&self, ips: Vec<String>) -> Result<Vec<GeoIpEntry>, DomainError> {
            Ok(ips.into_iter().map(|ip| GeoIpEntry {
                query: ip,
                status: "success".into(),
                country: Some("FR".into()),
                country_code: Some("FR".into()),
                region_name: None,
                city: Some("Paris".into()),
                isp: None,
                asn: None,
            }).collect())
        }
    }

    #[tokio::test]
    async fn delegates_to_geoip_lookup() {
        let service = LookupGeoIpService::new(Arc::new(FakeGeoIpLookup));
        let result = service.lookup(vec!["1.2.3.4".into()]).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].query, "1.2.3.4");
    }

    #[test]
    fn service_can_be_created() {
        let _service = LookupGeoIpService::new(Arc::new(FakeGeoIpLookup));
    }
}
