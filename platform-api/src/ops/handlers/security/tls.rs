//! TLS : certificat de l'hote et erreurs de handshake.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::read_probe;
use crate::ops::{ApiError, AppState};
use platform_core::ops::domain::entities::host_probe::HostProbe;

// TLS handshake errors
#[derive(Debug, Serialize, Deserialize)]
pub struct TlsErrorEntry {
    pub timestamp: String,
    pub client: String,
    pub error: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TlsErrorsResponse {
    pub updated_at: String,
    pub total_24h: i64,
    pub entries: Vec<TlsErrorEntry>,
}

pub async fn tls_errors(
    State(state): State<AppState>,
) -> Result<Json<TlsErrorsResponse>, ApiError> {
    Ok(Json(read_probe(&state, HostProbe::TlsErrors).await?))
}

// Certificat TLS
#[derive(Debug, Serialize)]
pub struct TlsCertInfo {
    pub domain: String,
    pub issuer: String,
    pub subject: String,
    pub not_before: String,
    pub not_after: String,
    pub days_until_expiry: i64,
    pub is_expired: bool,
    pub is_warning: bool, // < 14 jours
}

pub async fn tls_cert(State(state): State<AppState>) -> Result<Json<TlsCertInfo>, ApiError> {
    let info = state.tls_cert_uc.read().await?;
    Ok(Json(TlsCertInfo {
        domain: info.domain,
        issuer: info.issuer,
        subject: info.subject,
        not_before: info.not_before,
        not_after: info.not_after,
        days_until_expiry: info.days_until_expiry,
        is_expired: info.is_expired,
        is_warning: info.is_warning,
    }))
}
