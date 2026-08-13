//! GeoIP : resolution d'un lot d'IPs (via le use case `LookupGeoIpUseCase`).

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::ops::{ApiError, AppState};

#[derive(Debug, Deserialize)]
pub struct GeoIpQuery {
    /// IPs separees par virgule, max 100
    pub ips: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoIpEntry {
    pub query: String,
    pub status: String,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default, rename = "countryCode")]
    pub country_code: Option<String>,
    #[serde(default)]
    pub region_name: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub isp: Option<String>,
    #[serde(default, rename = "as")]
    pub asn: Option<String>,
}

pub async fn geoip_lookup(
    State(state): State<AppState>,
    Query(q): Query<GeoIpQuery>,
) -> Result<Json<Vec<GeoIpEntry>>, ApiError> {
    let ips: Vec<String> = q
        .ips
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .take(100)
        .map(|s| s.to_string())
        .collect();

    let rows = state.geoip_uc.lookup(ips).await?;
    Ok(Json(
        rows.into_iter()
            .map(|e| GeoIpEntry {
                query: e.query,
                status: e.status,
                country: e.country,
                country_code: e.country_code,
                region_name: e.region_name,
                city: e.city,
                isp: e.isp,
                asn: e.asn,
            })
            .collect(),
    ))
}
