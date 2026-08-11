//! Bans : export fail2ban (lecture), ban/unban manuels et leur liste.

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::{actor_from, record_event};
use crate::{ApiError, AppState};

// ── IPs bannies (lecture fichier export fail2ban) ───────────────────────

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
