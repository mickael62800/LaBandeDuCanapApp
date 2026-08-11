//! Sondes de l'hote : fichiers JSON exposes par les cron host (SSH, disque,
//! connexions, ports, Trivy, integrite, sorties reseau, patterns nginx).

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::read_probe;
use crate::{ApiError, AppState};
use ops_core::domain::entities::host_probe::HostProbe;

// SSH failures
#[derive(Debug, Serialize, Deserialize)]
pub struct SshFailureEntry {
    pub timestamp: String,
    pub user: String,
    pub ip: String,
    pub message: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SshFailuresResponse {
    pub updated_at: String,
    pub total_24h: i64,
    pub entries: Vec<SshFailureEntry>,
}

pub async fn ssh_failures(
    State(state): State<AppState>,
) -> Result<Json<SshFailuresResponse>, ApiError> {
    Ok(Json(read_probe(&state, HostProbe::SshFailures).await?))
}

// Disk trend
#[derive(Debug, Serialize, Deserialize)]
pub struct DiskTrendPoint {
    pub timestamp: String,
    pub mount: String,
    pub used_gb: f64,
    pub total_gb: f64,
    pub usage_pct: f64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DiskTrendResponse {
    pub updated_at: String,
    pub points: Vec<DiskTrendPoint>,
}

pub async fn disk_trend(
    State(state): State<AppState>,
) -> Result<Json<DiskTrendResponse>, ApiError> {
    Ok(Json(read_probe(&state, HostProbe::DiskTrend).await?))
}

// Active connections
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionEntry {
    pub state: String,
    pub local_addr: String,
    pub remote_addr: String,
    pub process: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionsResponse {
    pub updated_at: String,
    pub total: i64,
    pub connections: Vec<ConnectionEntry>,
}

pub async fn active_connections(
    State(state): State<AppState>,
) -> Result<Json<ConnectionsResponse>, ApiError> {
    Ok(Json(read_probe(&state, HostProbe::Connections).await?))
}

// Open ports check
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenPort {
    pub port: i64,
    pub protocol: String,
    pub service: Option<String>,
    pub expected: bool, // true si dans la liste blanche (80,443,22/2222)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenPortsResponse {
    pub updated_at: String,
    pub ports: Vec<OpenPort>,
    pub unexpected_count: i64,
}

pub async fn open_ports(
    State(state): State<AppState>,
) -> Result<Json<OpenPortsResponse>, ApiError> {
    Ok(Json(read_probe(&state, HostProbe::OpenPorts).await?))
}

// Trivy vulns
#[derive(Debug, Serialize, Deserialize)]
pub struct TrivyVuln {
    pub image: String,
    pub cve: String,
    pub severity: String, // CRITICAL / HIGH / MEDIUM / LOW
    pub package: Option<String>,
    pub fixed_version: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TrivyResponse {
    pub updated_at: String,
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
    pub vulnerabilities: Vec<TrivyVuln>,
}

pub async fn trivy_vulns(State(state): State<AppState>) -> Result<Json<TrivyResponse>, ApiError> {
    Ok(Json(read_probe(&state, HostProbe::Trivy).await?))
}

// File integrity
#[derive(Debug, Serialize, Deserialize)]
pub struct FileIntegrityEntry {
    pub path: String,
    pub sha256: String,
    pub modified_at: String,
    pub status: String, // "ok" | "modified" | "missing"
}
#[derive(Debug, Serialize, Deserialize)]
pub struct FileIntegrityResponse {
    pub updated_at: String,
    pub baseline_at: Option<String>,
    pub modified_count: i64,
    pub files: Vec<FileIntegrityEntry>,
}

pub async fn file_integrity(
    State(state): State<AppState>,
) -> Result<Json<FileIntegrityResponse>, ApiError> {
    Ok(Json(read_probe(&state, HostProbe::FileIntegrity).await?))
}

// Outbound connections
#[derive(Debug, Serialize, Deserialize)]
pub struct OutboundConnection {
    pub local_addr: String,
    pub remote_addr: String,
    pub remote_host: Option<String>,
    pub process: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct OutboundResponse {
    pub updated_at: String,
    pub total: i64,
    pub connections: Vec<OutboundConnection>,
}

pub async fn outbound_connections(
    State(state): State<AppState>,
) -> Result<Json<OutboundResponse>, ApiError> {
    Ok(Json(read_probe(&state, HostProbe::Outbound).await?))
}

// Nginx suspicious patterns
#[derive(Debug, Serialize, Deserialize)]
pub struct SuspiciousEntry {
    pub timestamp: String,
    pub ip: String,
    pub method: String,
    pub url: String,
    pub status: i64,
    pub category: String, // "scanner" | "sqli" | "xss" | "path-traversal"
    pub user_agent: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SuspiciousResponse {
    pub updated_at: String,
    pub total_24h: i64,
    pub by_category: serde_json::Value,
    pub entries: Vec<SuspiciousEntry>,
}

pub async fn nginx_suspicious(
    State(state): State<AppState>,
) -> Result<Json<SuspiciousResponse>, ApiError> {
    let mut data: SuspiciousResponse = read_probe(&state, HostProbe::NginxSuspicious).await?;

    // Filtre les entries dont l'IP est actuellement bannie manuellement.
    // Le fichier JSON est regenere par cron host depuis access.log et ne
    // tient pas compte des bans, donc on filtre ici. Les compteurs
    // (total_24h, by_category) sont rafraichis a partir des entries
    // restantes pour rester coherents avec ce que l'admin voit.
    let banned: Vec<String> = state
        .ip_bans_uc
        .list_manual_bans()
        .await
        .map(|bans| bans.into_iter().map(|b| b.ip).collect())
        .unwrap_or_default();

    if !banned.is_empty() {
        let banset: std::collections::HashSet<&str> = banned.iter().map(|s| s.as_str()).collect();
        data.entries.retain(|e| !banset.contains(e.ip.as_str()));
        data.total_24h = data.entries.len() as i64;
        let mut by_cat = std::collections::HashMap::<String, i64>::new();
        for e in &data.entries {
            *by_cat.entry(e.category.clone()).or_insert(0) += 1;
        }
        data.by_category = serde_json::to_value(by_cat).unwrap_or(serde_json::json!({}));
    }

    Ok(Json(data))
}
