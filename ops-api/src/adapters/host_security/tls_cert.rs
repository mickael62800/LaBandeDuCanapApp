//! Adapter du port `TlsCertReader` : recupere le cert du domaine web (lecture
//! fichier certbot, fallback `openssl s_client`) et en extrait l'expiration.

use async_trait::async_trait;

use ops_core::domain::entities::tls_cert::TlsCertInfo;
use ops_core::domain::errors::DomainError;
use ops_core::ports::outbound::tls_cert_reader::TlsCertReader;

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

        // 2 strategies :
        //   1. Lecture fichier /etc/letsencrypt/live/{domain}/cert.pem (rapide).
        //   2. Fallback : openssl s_client connect web:443 (independant des
        //      perms fichier certbot).
        let path = format!("/etc/letsencrypt/live/{domain}/cert.pem");
        let pem = match std::fs::read_to_string(&path) {
            Ok(p) => p,
            Err(_) => fetch_cert_via_openssl(&domain).map_err(|e| {
                DomainError::Internal(format!(
                    "lecture cert {path} echouee + fallback openssl echec : {e}"
                ))
            })?,
        };

        parse_cert(&pem).map_err(|e| DomainError::Internal(format!("parse cert: {e}")))
    }
}

/// Fallback : `openssl s_client -connect web:443 -servername {domain}` pour
/// recuperer le cert via TLS handshake (independant des perms fichier).
fn fetch_cert_via_openssl(domain: &str) -> Result<String, String> {
    use std::io::Write;
    use std::process::Command;
    use std::process::Stdio;

    let mut child = Command::new("openssl")
        .args([
            "s_client",
            "-connect",
            "web:443",
            "-servername",
            domain,
            "-showcerts",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn openssl: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"");
    }

    let output = child
        .wait_with_output()
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
