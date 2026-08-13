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
//! # Deux interrupteurs, et c'est voulu
//!
//! Le transfert en clair n'est plus un effet de bord d'`OPS_GEOIP_ENABLED`. Il
//! demande sa PROPRE declaration, `OPS_GEOIP_ALLOW_PLAINTEXT=true` :
//!
//!   - « je veux enrichir les IP » et « j'accepte que ces IP circulent en clair
//!     jusqu'a un tiers » sont deux decisions distinctes, dont la seconde a une
//!     portee RGPD ;
//!   - le defaut publie etant en `http://`, activer la resolution suffisait a
//!     declencher le transfert en clair sans que rien ne le dise. Un exploitant
//!     qui pointe la variable sur un service TLS n'a, lui, rien a declarer.
//!
//! Sans cette declaration, une URL non-TLS desactive la resolution — l'ecran
//! affiche les IP sans pays, comme lorsqu'elle est simplement eteinte. On ne
//! degrade jamais silencieusement vers l'envoi en clair.
//!
//! Le palier gratuit est en outre limite a 45 requetes/minute, et le depasser
//! fait bannir l'IP de l'hote : un depassement est donc traite comme une erreur
//! nommee, pas comme un echec generique.

use async_trait::async_trait;

use platform_core::ops::domain::entities::geoip::GeoIpEntry;
use platform_core::ops::domain::errors::DomainError;
use platform_core::ops::ports::outbound::geoip_lookup::GeoIpLookup;

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
        // Fail-closed sur la donnee personnelle : sans opt-in explicite,
        // aucune IP ne sort.
        let demande = std::env::var("OPS_GEOIP_ENABLED")
            .map(|v| platform_common::config_flags::parse_bool_str(&v))
            .unwrap_or(false);
        let url = std::env::var("OPS_GEOIP_URL")
            .unwrap_or_else(|_| "http://ip-api.com/batch".to_string());
        let clair_autorise = std::env::var("OPS_GEOIP_ALLOW_PLAINTEXT")
            .map(|v| platform_common::config_flags::parse_bool_str(&v))
            .unwrap_or(false);

        // L'etat est fige au demarrage, comme les autres reglages lus dans
        // l'environnement : la decision ne doit pas dependre du moment ou la
        // premiere IP est resolue.
        let enabled = demande && transport_acceptable(&url, clair_autorise);
        if demande && !enabled {
            tracing::warn!(
                url = %url,
                "resolution GeoIP DESACTIVEE : l'URL n'est pas en https et \
                 OPS_GEOIP_ALLOW_PLAINTEXT n'est pas declaree. Les adresses IP \
                 des visiteurs circuleraient en clair jusqu'a un tiers."
            );
        }

        Self { enabled, url }
    }
}

/// Le transport est-il acceptable pour y faire passer des adresses IP ?
///
/// Isole et pur : c'est la regle qu'on veut pouvoir verifier sans monter
/// l'adaptateur ni toucher a l'environnement.
///
/// Une URL non analysable est refusee — dans le doute sur ce qu'on s'apprete a
/// contacter, on ne contacte pas.
///
/// `reqwest::Url` plutot qu'une comparaison de prefixe : le schema doit etre
/// lu comme le fera le client HTTP, pas comme on l'imagine. Aucune dependance
/// nouvelle — reqwest est deja la, juste en dessous.
fn transport_acceptable(url: &str, clair_autorise: bool) -> bool {
    match reqwest::Url::parse(url.trim()) {
        Ok(u) => match u.scheme() {
            "https" => true,
            "http" => clair_autorise,
            // Ni http ni https : `reqwest` echouerait de toute facon, autant
            // le dire au demarrage plutot qu'a la premiere resolution.
            _ => false,
        },
        Err(_) => false,
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
            // Trois causes possibles, toutes tracees au demarrage :
            // `OPS_GEOIP_ENABLED` absente, URL non-TLS sans declaration, ou URL
            // inexploitable.
            tracing::debug!("resolution GeoIP desactivee");
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

#[cfg(test)]
mod tests_transport {
    use super::transport_acceptable;

    #[test]
    fn https_passe_sans_rien_declarer() {
        assert!(transport_acceptable("https://ip-api.com/batch", false));
    }

    #[test]
    fn http_exige_la_declaration_explicite() {
        // Le defaut publie : activer la resolution ne doit plus suffire.
        assert!(!transport_acceptable("http://ip-api.com/batch", false));
        assert!(transport_acceptable("http://ip-api.com/batch", true));
    }

    #[test]
    fn un_schema_inutilisable_est_refuse_meme_avec_la_declaration() {
        // `OPS_GEOIP_ALLOW_PLAINTEXT` autorise le clair, pas n'importe quoi.
        assert!(!transport_acceptable("ftp://ip-api.com/batch", true));
        assert!(!transport_acceptable("pas-une-url", true));
        assert!(!transport_acceptable("", true));
    }
}
