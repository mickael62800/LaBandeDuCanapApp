//! Resultat d'une resolution GeoIP (panel securite : localiser les IPs).

/// Geolocalisation d'une IP (champs optionnels selon le fournisseur).
#[derive(Debug, Clone)]
pub struct GeoIpEntry {
    pub query: String,
    pub status: String,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub region_name: Option<String>,
    pub city: Option<String>,
    pub isp: Option<String>,
    pub asn: Option<String>,
}
