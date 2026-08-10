//! Adapter du port `GeoIpLookup` : resolution batch via ip-api.com
//! (gratuit, 45 req/min). Appel HTTP isole de l'adapter HTTP inbound.

use async_trait::async_trait;

use ops_core::domain::entities::geoip::GeoIpEntry;
use ops_core::domain::errors::DomainError;
use ops_core::ports::outbound::geoip_lookup::GeoIpLookup;

/// Forme renvoyee par ip-api.com (champs renommes -> snake_case domaine).
#[derive(serde::Deserialize)]
struct RawGeoIp {
    query: String,
    status: String,
    #[serde(default)]
    country: Option<String>,
    #[serde(default, rename = "countryCode")]
    country_code: Option<String>,
    #[serde(default, rename = "regionName")]
    region_name: Option<String>,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    isp: Option<String>,
    #[serde(default, rename = "as")]
    asn: Option<String>,
}

#[derive(Default)]
pub struct IpApiGeoIpLookup;

impl IpApiGeoIpLookup {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GeoIpLookup for IpApiGeoIpLookup {
    async fn lookup(&self, ips: Vec<String>) -> Result<Vec<GeoIpEntry>, DomainError> {
        if ips.is_empty() {
            return Ok(vec![]);
        }

        let body: Vec<serde_json::Value> = ips
            .iter()
            .map(|ip| {
                serde_json::json!({
                    "query": ip,
                    "fields": "status,country,countryCode,regionName,city,isp,as,query"
                })
            })
            .collect();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| DomainError::Internal(format!("reqwest build: {e}")))?;

        let resp = client
            .post("http://ip-api.com/batch")
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("geoip lookup: {e}")))?;
        let raw: Vec<RawGeoIp> = resp
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("geoip parse: {e}")))?;

        Ok(raw
            .into_iter()
            .map(|r| GeoIpEntry {
                query: r.query,
                status: r.status,
                country: r.country,
                country_code: r.country_code,
                region_name: r.region_name,
                city: r.city,
                isp: r.isp,
                asn: r.asn,
            })
            .collect())
    }
}
