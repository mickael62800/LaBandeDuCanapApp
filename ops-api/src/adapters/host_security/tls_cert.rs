//! Adapter du port `TlsCertReader` : recupere le cert du domaine web via le
//! handshake TLS et en extrait l'expiration.
//!
//! # Pourquoi le handshake et PAS le fichier certbot
//!
//! Une premiere version lisait `/etc/letsencrypt/live/{domain}/cert.pem`, ce
//! qui imposait de monter `/etc/letsencrypt` dans ce conteneur. Or `privkey.pem`
//! vit dans le meme repertoire : une compromission d'ops-api livrait la CLE
//! PRIVEE du certificat, pour n'afficher qu'une date d'expiration. Restreindre
//! le montage a `live/` n'aurait rien change — certbot n'y met que des liens
//! symboliques vers `archive/`, qui contient les cles aussi.
//!
//! Le handshake rend exactement la meme information et ne lit qu'une donnee
//! publique : celle que le certificat presente a n'importe quel visiteur. Le
//! montage a donc ete retire du compose.

use std::time::Duration;

use async_trait::async_trait;

use ops_core::domain::entities::tls_cert::TlsCertInfo;
use ops_core::domain::errors::DomainError;
use ops_core::ports::outbound::tls_cert_reader::TlsCertReader;

/// Plafond de l'appel `openssl`. Sans lui, un `web:443` qui accepte la
/// connexion sans jamais repondre laisse le processus en attente indefinie —
/// et, l'appel etant fait depuis un handler, immobilise un thread du runtime.
const OPENSSL_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Default)]
pub struct FileTlsCertReader;

impl FileTlsCertReader {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TlsCertReader for FileTlsCertReader {
    async fn read(&self) -> Result<TlsCertInfo, DomainError> {
        let domain = std::env::var("WEB_DOMAIN").unwrap_or_default();
        if domain.is_empty() {
            return Err(DomainError::Internal("WEB_DOMAIN non defini en env".into()));
        }

        let pem = fetch_cert_via_openssl(&domain)
            .await
            .map_err(DomainError::Internal)?;

        parse_cert(&pem).map_err(|e| DomainError::Internal(format!("parse cert: {e}")))
    }
}

/// `openssl s_client -connect web:443 -servername {domain}` : recupere le cert
/// tel qu'il est presente au handshake.
///
/// `tokio::process` et non `std::process` : la version bloquante retenait un
/// thread du runtime pendant toute la duree de l'appel.
async fn fetch_cert_via_openssl(domain: &str) -> Result<String, String> {
    use tokio::process::Command;

    // `-servername` est passe en argument d'un exec direct, jamais a un shell :
    // aucune interpolation possible, meme si WEB_DOMAIN etait mal renseigne.
    let child = Command::new("openssl")
        .args([
            "s_client",
            "-connect",
            "web:443",
            "-servername",
            domain,
            "-showcerts",
        ])
        // `< /dev/null` : sans fermeture de l'entree, `s_client` reste ouvert
        // en attente d'input apres le handshake.
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("spawn openssl: {e}"))?;

    let output = tokio::time::timeout(OPENSSL_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| format!("openssl n'a pas repondu en {}s", OPENSSL_TIMEOUT.as_secs()))?
        .map_err(|e| format!("wait openssl: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let begin = stdout
        .find("-----BEGIN CERTIFICATE-----")
        .ok_or_else(|| "BEGIN CERTIFICATE marker absent".to_string())?;
    let end_marker = "-----END CERTIFICATE-----";
    let end = stdout[begin..]
        .find(end_marker)
        .ok_or_else(|| "END CERTIFICATE marker absent".to_string())?;
    let pem = &stdout[begin..begin + end + end_marker.len()];
    Ok(pem.to_string())
}

fn parse_cert(pem: &str) -> Result<TlsCertInfo, String> {
    use x509_parser::pem::parse_x509_pem;
    use x509_parser::prelude::*;

    let (_, p) = parse_x509_pem(pem.as_bytes()).map_err(|e| format!("pem: {e}"))?;
    let (_, cert) = X509Certificate::from_der(&p.contents).map_err(|e| format!("der: {e}"))?;

    let issuer = cert.issuer().to_string();
    let subject = cert.subject().to_string();
    let nb = cert.validity().not_before;
    let na = cert.validity().not_after;

    let not_before = nb.to_rfc2822().unwrap_or_else(|_| nb.to_string());
    let not_after = na.to_rfc2822().unwrap_or_else(|_| na.to_string());

    let now = chrono::Utc::now();
    let na_chrono = chrono::DateTime::<chrono::Utc>::from_timestamp(na.timestamp(), 0)
        .ok_or_else(|| "timestamp invalide".to_string())?;
    let days_until_expiry = (na_chrono - now).num_days();
    let is_expired = days_until_expiry < 0;
    let is_warning = !is_expired && days_until_expiry < 14;

    let domain = cert
        .subject()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("")
        .to_string();

    Ok(TlsCertInfo {
        domain,
        issuer,
        subject,
        not_before,
        not_after,
        days_until_expiry,
        is_expired,
        is_warning,
    })
}
