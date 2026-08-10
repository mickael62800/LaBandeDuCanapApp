//! Info d'expiration du certificat TLS du domaine web (panel securite).

/// Informations extraites du certificat TLS courant.
#[derive(Debug, Clone)]
pub struct TlsCertInfo {
    pub domain: String,
    pub issuer: String,
    pub subject: String,
    pub not_before: String,
    pub not_after: String,
    pub days_until_expiry: i64,
    pub is_expired: bool,
    /// Expire dans moins de 14 jours (et pas encore expire).
    pub is_warning: bool,
}
