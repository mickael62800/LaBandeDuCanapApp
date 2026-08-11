//! GET /api/security/* — surveillance attaques et integrite serveur.
//!
//! Tous les endpoints sont gates admin+ (require_role).
//! Sources :
//!   - logs : table `logs` (alimentee par api_logger_middleware)
//!   - audit_logs : table `audit_logs` (Discord events + extension audit_docker)
//!   - cert TLS : lecture du fichier /etc/letsencrypt/live/{domain}/cert.pem
//!   - fail2ban : non implemente (necessite installation host)

use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::ApiError;

use crate::AppState;
use ops_core::domain::entities::host_probe::HostProbe;
use ops_core::domain::entities::security_audit::{AuditLogFilter, CleanupOptions};
use ops_core::domain::entities::security_log::LogWindow;

// ── 1. Top IPs par requetes ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WindowQuery {
    /// "1h" / "24h" / "7d", defaut 1h
    pub window: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TopIpEntry {
    pub client_ip: String,
    pub total: i64,
    pub failed: i64,
    pub last_seen: String,
}

pub async fn top_ips(
    State(state): State<AppState>,
    Query(q): Query<WindowQuery>,
) -> Result<Json<Vec<TopIpEntry>>, ApiError> {
    let window = LogWindow::parse(q.window.as_deref().unwrap_or("1h"));
    let limit = crate::handlers::normalize_in(q.limit, 20, 1, 100);

    let rows = state.security_logs_uc.top_ips(window, limit).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| TopIpEntry {
                client_ip: r.client_ip,
                total: r.total,
                failed: r.failed,
                last_seen: r.last_seen.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            })
            .collect(),
    ))
}

// ── 2. Echecs d'auth (401/403) recents ──────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AuthFailureEntry {
    pub timestamp: String,
    pub status_code: i64,
    pub method: String,
    pub route: String,
    pub client_ip: String,
    pub user_agent: String,
}

pub async fn auth_failures(
    State(state): State<AppState>,
    Query(q): Query<WindowQuery>,
) -> Result<Json<Vec<AuthFailureEntry>>, ApiError> {
    let window = LogWindow::parse(q.window.as_deref().unwrap_or("24h"));
    let limit = crate::handlers::normalize_in(q.limit, 100, 1, 500);

    let rows = state.security_logs_uc.auth_failures(window, limit).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| AuthFailureEntry {
                timestamp: r.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                status_code: r.status_code,
                method: r.method,
                route: r.route,
                client_ip: r.client_ip,
                user_agent: r.user_agent,
            })
            .collect(),
    ))
}

// ── 3. IPs bannies (lecture fichier export fail2ban) ───────────────────

#[derive(Debug, Serialize)]
pub struct Fail2banJail {
    pub name: String,
    pub total_banned: i64,
    pub banned_ips: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BannedIpsResponse {
    pub installed: bool,
    pub updated_at: Option<String>,
    pub message: String,
    pub jails: Vec<Fail2banJail>,
}

pub async fn banned_ips(
    State(state): State<AppState>,
) -> Result<Json<BannedIpsResponse>, ApiError> {
    let Some(status) = state.ip_bans_uc.fail2ban_status().await? else {
        return Ok(Json(BannedIpsResponse {
            installed: false,
            updated_at: None,
            message: "fail2ban status non disponible. Pour activer : 1) installer fail2ban sur l'host (apt install fail2ban) ; 2) creer le script /usr/local/bin/fail2ban-export.sh + cron pour exporter dans /var/lib/sentinel/fail2ban-status.json ; 3) monter /var/lib/sentinel:/var/lib/sentinel:ro dans le conteneur api du compose.".to_string(),
            jails: vec![],
        }));
    };

    let total = status.total_banned_ips();
    let jails: Vec<Fail2banJail> = status
        .jails
        .into_iter()
        .map(|j| Fail2banJail {
            name: j.name,
            total_banned: j.total_banned,
            banned_ips: j.banned_ips,
        })
        .collect();
    Ok(Json(BannedIpsResponse {
        installed: true,
        updated_at: Some(status.updated_at),
        message: format!(
            "{} IPs actuellement bannies sur {} jail(s)",
            total,
            jails.len()
        ),
        jails,
    }))
}

// ── 4. Audit log admin (actions sensibles) ──────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub guild_id: Option<String>,
    pub event_type_prefix: Option<String>, // ex: "docker." ou "user."
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: String,
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub details: serde_json::Value,
    pub created_at: String,
}

pub async fn audit_logs(
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEntry>>, ApiError> {
    let filter = AuditLogFilter {
        guild_id: q.guild_id,
        event_type_prefix: q.event_type_prefix,
        limit: crate::handlers::normalize_in(q.limit, 100, 1, 500),
    };
    let rows = state.security_audit_uc.audit_logs(filter).await?;
    Ok(Json(
        rows.into_iter()
            .map(|e| AuditEntry {
                id: e.id,
                guild_id: e.guild_id,
                event_type: e.event_type,
                actor_id: e.actor_id,
                actor_name: e.actor_name,
                target_id: e.target_id,
                target_name: e.target_name,
                details: e.details,
                created_at: e.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            })
            .collect(),
    ))
}

// ── Ban IP : ajoute une IP a la blocklist host ──────────────────────────

#[derive(Debug, Deserialize)]
pub struct BanIpDto {
    pub ip: String,
    /// Optionnel : raison libre (ex: "trop d'echecs auth")
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BanIpResponse {
    pub ok: bool,
    pub message: String,
}

/// POST /api/security/ban-ip
/// Delegue au use case `ManageIpBansUseCase` (validation + file-shim host +
/// persistance + purge logs). Le handler ne fait que le gate, l'audit et le
/// mapping de la reponse.
pub async fn ban_ip(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<BanIpDto>,
) -> Result<Json<BanIpResponse>, ApiError> {
    let actor = actor_from(&headers);

    let outcome = state
        .ip_bans_uc
        .ban(&dto.ip, dto.reason.clone(), &actor)
        .await?;

    let ip = dto.ip.trim();
    record_event(
        &state.server_events,
        &actor,
        None,
        "security.ban_ip",
        Some(ip),
        "warn",
        serde_json::json!({ "reason": dto.reason, "ip": ip, "deleted_logs": outcome.deleted_logs }),
    )
    .await;

    Ok(Json(BanIpResponse {
        ok: true,
        message: format!(
            "IP {} bannie ({} logs purges, sera applique au prochain tick du cron host)",
            ip, outcome.deleted_logs
        ),
    }))
}

// ── Unban IP : retire une IP de la blocklist ────────────────────────────

pub async fn unban_ip(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<BanIpDto>,
) -> Result<Json<BanIpResponse>, ApiError> {
    let actor = actor_from(&headers);

    state
        .ip_bans_uc
        .unban(&dto.ip, dto.reason.clone(), &actor)
        .await?;

    let ip = dto.ip.trim();
    record_event(
        &state.server_events,
        &actor,
        None,
        "security.unban_ip",
        Some(ip),
        "info",
        serde_json::json!({ "reason": dto.reason, "ip": ip }),
    )
    .await;

    Ok(Json(BanIpResponse {
        ok: true,
        message: format!(
            "IP {} retiree de la blocklist (sera applique au prochain tick)",
            ip
        ),
    }))
}

// ── Lecture de fichiers JSON exposes par les cron host ──────────────────

/// Helper : lit une sonde host via le use case et la deserialise dans le DTO
/// de reponse. Toute l'infra (fichier, chemin) est dans l'adapter outbound.
async fn read_probe<T: for<'de> serde::Deserialize<'de>>(
    state: &AppState,
    probe: HostProbe,
) -> Result<T, ApiError> {
    let value = state.host_probe_uc.read(probe).await?;
    serde_json::from_value(value).map_err(|e| {
        ApiError(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("parse {}: {e}", probe.feature()),
        )
    })
}

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

// GeoIP lookup via ip-api.com (batch, gratuit 45 req/min)
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

// Container changes (snapshot diff via bollard)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerSnapshot {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub started_at: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerChangeEntry {
    pub timestamp: String,
    pub kind: String, // "added" | "removed" | "restarted" | "image_changed" | "state_changed"
    pub container: ContainerSnapshot,
    pub previous: Option<ContainerSnapshot>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerChangesResponse {
    pub last_check: String,
    pub current: Vec<ContainerSnapshot>,
    pub changes_24h: Vec<ContainerChangeEntry>,
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

// ── Manual bans : liste des bans declenches via panel ──────────────────

#[derive(Debug, Serialize)]
pub struct ManualBanEntry {
    pub ip: String,
    pub banned_at: String,
    pub banned_by: Option<String>,
    pub reason: Option<String>,
}

pub async fn manual_bans(
    State(state): State<AppState>,
) -> Result<Json<Vec<ManualBanEntry>>, ApiError> {
    let bans = state.ip_bans_uc.list_manual_bans().await?;
    Ok(Json(
        bans.into_iter()
            .map(|b| ManualBanEntry {
                ip: b.ip,
                banned_at: b.banned_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                banned_by: b.banned_by,
                reason: b.reason,
            })
            .collect(),
    ))
}

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

// ── Last successful logins ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SuccessfulLoginEntry {
    pub timestamp: String,
    pub discord_user_id: String,
    pub username: Option<String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

pub async fn last_successful_logins(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Vec<SuccessfulLoginEntry>>, ApiError> {
    let limit = crate::handlers::normalize_in(q.limit, 20, 1, 200);
    let rows = state.security_audit_uc.recent_logins(limit).await?;
    Ok(Json(
        rows.into_iter()
            .map(|l| SuccessfulLoginEntry {
                timestamp: l.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                discord_user_id: l.discord_user_id,
                username: l.username,
                client_ip: l.client_ip,
                user_agent: l.user_agent,
            })
            .collect(),
    ))
}

// ── Trafic anormal : graphe req/s sur N heures ─────────────────────────

#[derive(Debug, Deserialize)]
pub struct TrafficTrendQuery {
    /// Fenetre : "1h", "6h", "24h", "7d"
    pub window: Option<String>,
    /// Bucket : taille en minutes (5 par defaut)
    pub bucket_minutes: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct TrafficDatapoint {
    pub timestamp: String,
    pub total: i64,
    pub errors: i64, // 4xx + 5xx
}

#[derive(Debug, Serialize)]
pub struct TrafficTrendResponse {
    pub datapoints: Vec<TrafficDatapoint>,
    pub baseline_avg: f64,
    pub peak: i64,
    pub peak_at: Option<String>,
    pub alert: bool,
    pub alert_reason: Option<String>,
}

pub async fn traffic_trend(
    State(state): State<AppState>,
    Query(q): Query<TrafficTrendQuery>,
) -> Result<Json<TrafficTrendResponse>, ApiError> {
    let window = LogWindow::parse(q.window.as_deref().unwrap_or("24h"));
    let bucket_min = crate::handlers::normalize_in(q.bucket_minutes.map(i64::from), 5, 1, 60);

    let trend = state
        .security_logs_uc
        .traffic_trend(window, bucket_min)
        .await?;

    Ok(Json(TrafficTrendResponse {
        datapoints: trend
            .datapoints
            .into_iter()
            .map(|d| TrafficDatapoint {
                timestamp: d.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                total: d.total,
                errors: d.errors,
            })
            .collect(),
        baseline_avg: trend.baseline_avg,
        peak: trend.peak,
        peak_at: trend
            .peak_at
            .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        alert: trend.alert,
        alert_reason: trend.alert_reason,
    }))
}

// ── Cleanup : purge des logs de securite ────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CleanupQuery {
    /// Nb de jours a garder. 0 = tout supprimer. Defaut 0.
    #[serde(default)]
    pub older_than_days: Option<i64>,
    /// True = purger les logs API (Top IPs, auth failures). Defaut true.
    #[serde(default)]
    pub include_api_logs: Option<bool>,
    /// True = purger aussi audit_logs (events Discord). Defaut false.
    #[serde(default)]
    pub include_audit_logs: Option<bool>,
    /// Purge `server_events` (audit infra : ban-ip, docker, user).
    #[serde(default)]
    pub include_server_events: Option<bool>,
    /// Purge `successful_logins` (derniers logins OAuth Discord).
    #[serde(default)]
    pub include_successful_logins: Option<bool>,
    /// Purge `manual_ip_bans` (historique des bans, incl. ceux deja leves).
    #[serde(default)]
    pub include_manual_bans: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CleanupResponse {
    pub deleted_api_logs: i64,
    pub deleted_audit_logs: i64,
    pub deleted_server_events: i64,
    pub deleted_successful_logins: i64,
    pub deleted_manual_bans: i64,
    pub message: String,
}

/// DELETE /api/security/cleanup
/// Supprime les entrees de logs (table `logs` cat='api') et optionnellement
/// `audit_logs`. Gate superadmin uniquement (operation destructive).
pub async fn cleanup_security_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CleanupQuery>,
) -> Result<Json<CleanupResponse>, ApiError> {
    // Endpoint cross-guild ultra-destructif (peut DELETE FROM audit_logs
    // global). L'acces est deja garde par la passerelle nginx, qui n'admet
    // que les superadmins : ops-api n'a pas de notion d'utilisateur a
    // reverifier.

    let options = CleanupOptions {
        older_than_days: q.older_than_days.unwrap_or(0).max(0),
        include_api_logs: q.include_api_logs.unwrap_or(true),
        include_audit_logs: q.include_audit_logs.unwrap_or(false),
        include_server_events: q.include_server_events.unwrap_or(false),
        include_successful_logins: q.include_successful_logins.unwrap_or(false),
        include_manual_bans: q.include_manual_bans.unwrap_or(false),
    };

    let report = state.security_audit_uc.cleanup(options.clone()).await?;

    let actor = actor_from(&headers);
    tracing::info!(
        target: "audit::security",
        actor = actor,
        api_logs = report.deleted_api_logs,
        audit_logs = report.deleted_audit_logs,
        server_events = report.deleted_server_events,
        successful_logins = report.deleted_successful_logins,
        manual_bans = report.deleted_manual_bans,
        days_kept = options.older_than_days,
        "security cleanup executed"
    );
    record_event(
        &state.server_events,
        &actor,
        None,
        "security.cleanup",
        Some(&format!("days={}", options.older_than_days)),
        if options.include_audit_logs {
            "warn"
        } else {
            "info"
        },
        serde_json::json!({
            "deleted_api_logs": report.deleted_api_logs,
            "deleted_audit_logs": report.deleted_audit_logs,
            "deleted_server_events": report.deleted_server_events,
            "deleted_successful_logins": report.deleted_successful_logins,
            "deleted_manual_bans": report.deleted_manual_bans,
            "days_kept": options.older_than_days,
        }),
    )
    .await;

    Ok(Json(CleanupResponse {
        deleted_api_logs: report.deleted_api_logs as i64,
        deleted_audit_logs: report.deleted_audit_logs as i64,
        deleted_server_events: report.deleted_server_events as i64,
        deleted_successful_logins: report.deleted_successful_logins as i64,
        deleted_manual_bans: report.deleted_manual_bans as i64,
        message: format!(
            "{} logs API, {} audit, {} events, {} logins, {} bans manuels supprimes",
            report.deleted_api_logs,
            report.deleted_audit_logs,
            report.deleted_server_events,
            report.deleted_successful_logins,
            report.deleted_manual_bans
        ),
    }))
}

// ── 5. Certificat TLS ───────────────────────────────────────────────────

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

/// Journalise une action d'exploitation, sans jamais la faire echouer.
///
/// Une action de securite qui a REUSSI ne doit pas remonter une erreur parce
/// que sa trace n'a pas pu s'ecrire : on prefere perdre la ligne de journal
/// que faire croire a l'operateur que le bannissement n'a pas eu lieu.
async fn record_event(
    repo: &std::sync::Arc<
        dyn ops_core::ports::outbound::server_event_repository::ServerEventRepository,
    >,
    actor: &str,
    actor_name: Option<&str>,
    action: &str,
    target: Option<&str>,
    severity: &str,
    details: serde_json::Value,
) {
    if let Err(error) = repo
        .record(actor, actor_name, action, target, severity, details)
        .await
    {
        tracing::warn!(%error, action, "journalisation d'un evenement impossible");
    }
}
/// Identifiant Discord de l'operateur, remonte par nginx (X-Actor-Id).
///
/// Sans cette remontee, l'audit des bannissements et des purges perdrait son
/// auteur : on saurait qu'une IP a ete bannie, jamais par qui.
fn actor_from(headers: &HeaderMap) -> String {
    headers
        .get("x-actor-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .unwrap_or("inconnu")
        .to_owned()
}
