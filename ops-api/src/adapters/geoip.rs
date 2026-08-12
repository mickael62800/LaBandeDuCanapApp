//! Adapter du port `GeoIpLookup` : resolution batch via un service externe.
//!
//! # Ce que cet adapter envoie dehors
//!
//! Des ADRESSES IP DE VISITEURS, a un tiers. C'est une donnee personnelle au
//! sens du RGPD, et le transfert doit donc etre un choix explicite : la
//! resolution est DESACTIVEE par defaut et s'active par
//! `OPS_GEOIP_ENABLED=true`. Desactivee, l'ecran affiche les IP sans pays
//! plutot que d'exporter la liste a l'insu de l'exploitant.
//!
//! Le point de terminaison est configurable (`OPS_GEOIP_URL`). Le defaut vise
//! ip-api.com, dont le palier gratuit n'accepte que **http://** : la requete et
//! sa reponse circulent en clair, donc un observateur du reseau voit quelles IP
//! sont enquetees. Pointer cette variable sur un service TLS (palier payant
//! d'ip-api, ou une instance auto-hebergee) supprime cette exposition ; une
//! base locale type GeoLite2 supprimerait le transfert lui-meme.
//!
//! Le palier gratuit est en outre limite a 45 requetes/minute, et le depasser
//! fait bannir l'IP de l'hote : un depassement est donc traite comme une erreur
//! nommee, pas comme un echec generique.

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

/// Le lot maximum accepte par ip-api.com. Au-dela, le service rejette la
/// requete entiere : on tronque plutot que de tout perdre.
const MAX_BATCH: usize = 100;

pub struct IpApiGeoIpLookup {
    enabled: bool,
    url: String,
}

impl Default for IpApiGeoIpLookup {
    fn default() -> Self {
        Self::new()
    }
}

impl IpApiGeoIpLookup {
    pub fn new() -> Self {
        Self {
            // Fail-closed sur la donnee personnelle : sans opt-in explicite,
            // aucune IP ne sort.
            enabled: std::env::var("OPS_GEOIP_ENABLED")
                .map(|v| platform_common::config_flags::parse_bool_str(&v))
                .unwrap_or(false),
            url: std::env::var("OPS_GEOIP_URL")
                .unwrap_or_else(|_| "http://ip-api.com/batch".to_string()),
        }
    }
}

#[async_trait]
impl GeoIpLookup for IpApiGeoIpLookup {
    async fn lookup(&self, ips: Vec<String>) -> Result<Vec<GeoIpEntry>, DomainError> {
        if ips.is_empty() {
            return Ok(vec![]);
        }
        if !self.enabled {
            // Pas une erreur : l'ecran doit afficher les IP sans enrichissement.
            tracing::debug!("resolution GeoIP desactivee (OPS_GEOIP_ENABLED)");
            return Ok(vec![]);
        }
        let ips: Vec<String> = ips.into_iter().take(MAX_BATCH).collect();

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
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("geoip lookup: {e}")))?;

        // 429 : le quota du palier gratuit (45 req/min) est atteint. Insister
        // fait bannir l'IP de l'hote par le fournisseur — on le nomme pour que
        // l'appelant recule au lieu de reessayer.
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(DomainError::RateLimited(
                "quota GeoIP atteint, reessayer dans une minute".into(),
            ));
        }
        if !resp.status().is_success() {
            return Err(DomainError::Internal(format!(
                "geoip: reponse {}",
                resp.status()
            )));
        }

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
