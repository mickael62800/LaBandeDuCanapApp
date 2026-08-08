//! Helper qui agrege la config bot game-portal pour une guild en une struct.
//!
//! Lit les valeurs via `BotConfigRepository::get_config(guild_id, "game-portal")`
//! et applique les defaults documentes en migration 189. Centralise pour
//! eviter de dupliquer les defaults dans chaque use case.

use std::sync::Arc;

use crate::domain::entities::game::server::{
    DEFAULT_MAX_RESTART_ATTEMPTS, DEFAULT_RESTART_BACKOFF_BASE_SECS,
    DEFAULT_RESTART_BACKOFF_CAP_SECS,
};
use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::domain::errors::DomainError;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

#[derive(Debug, Clone)]
pub struct GamePortalConfig {
    pub enabled: bool,
    pub max_servers_per_guild: i32,
    pub max_memory_total_mb: i32,
    pub port_range_start: u16,
    pub port_range_end: u16,
    pub rcon_port_range_start: u16,
    pub rcon_port_range_end: u16,
    pub allowed_templates: Vec<String>,
    pub default_idle_shutdown_days: i32,
    pub docker_network_name: String,
    pub container_user: String,
    pub host_data_dir: String,
    pub auto_create_world_volume: bool,
    pub rcon_enabled: bool,
    pub log_channel_id: Option<String>,
    /// Active la suppression auto des images Docker non utilisees.
    pub auto_remove_unused_images: bool,
    /// Nombre de jours sans aucun serveur actif avant suppression de l'image.
    pub unused_image_grace_days: i32,
    /// Timeout (secondes) d'une requete RCON avant abandon.
    pub rcon_timeout_secs: u32,
    /// Redemarrage auto des containers crashes (backoff exponentiel).
    pub auto_restart_on_crash: bool,
    /// Nombre max de redemarrages auto consecutifs avant abandon (-> Error).
    pub max_auto_restart_attempts: i32,
    /// Base (secondes) du backoff exponentiel entre deux redemarrages auto.
    pub restart_backoff_base_secs: i64,
    /// Plafond (secondes) du backoff exponentiel entre deux redemarrages auto.
    pub restart_backoff_cap_secs: i64,
    /// Delai (minutes) au-dela duquel un serveur coince dans un etat
    /// transitoire (Starting/Stopping) est resolu de force par le reconciler.
    pub stuck_transition_threshold_minutes: i64,
    /// Delai de grace (secondes) laisse a un container pour s'arreter proprement
    /// avant kill (docker stop -t).
    pub stop_timeout_secs: u32,
    /// Nombre max de lignes de logs recuperables en une requete (borne dure).
    pub max_log_lines: u32,
    // Sessions de jeu (salons Discord + revelation IP).
    pub session_category_id: Option<String>,
    pub ip_reveal_default_days: i32,
    pub session_daily_ping_enabled: bool,
    pub session_daily_ping_hour: i32,
}

fn find<'a>(items: &'a [BotGuildConfig], key: &str) -> Option<&'a str> {
    items
        .iter()
        .find(|c| c.config_key == key)
        .map(|c| c.config_value.as_str())
}

fn parse_bool(s: Option<&str>, default: bool) -> bool {
    match s.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
        Some("true") | Some("1") | Some("yes") | Some("on") => true,
        Some("false") | Some("0") | Some("no") | Some("off") => false,
        _ => default,
    }
}

fn parse_i32(s: Option<&str>, default: i32) -> i32 {
    s.and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn parse_u16(s: Option<&str>, default: u16) -> u16 {
    s.and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn parse_u32(s: Option<&str>, default: u32) -> u32 {
    s.and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn parse_i64(s: Option<&str>, default: i64) -> i64 {
    s.and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn parse_string(s: Option<&str>, default: &str) -> String {
    s.unwrap_or(default).to_string()
}

fn parse_csv(s: Option<&str>, default: &str) -> Vec<String> {
    let raw = s.unwrap_or(default);
    raw.split(',')
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Plus petit port non privilegie. En dessous (0-1023) les ports sont
/// reserves (root) et ne doivent jamais etre exposes par un container jeu.
const MIN_UNPRIVILEGED_PORT: u16 = 1024;

/// Valide/clamp un range de ports configure par l'admin :
///  - `start` est plancher a 1024 (jamais de port privilegie expose) ;
///  - `end` est ramene a `start` s'il est plus petit (range coherent).
/// Emet un warn a chaque correction. Point unique de validation des ranges.
fn sanitize_port_range(kind: &str, start: u16, end: u16) -> (u16, u16) {
    let mut s = start;
    let mut e = end;
    if s < MIN_UNPRIVILEGED_PORT {
        tracing::warn!(
            kind,
            configured_start = start,
            "port_range_start < 1024 (privilegie) : clamp a 1024"
        );
        s = MIN_UNPRIVILEGED_PORT;
    }
    if e < s {
        tracing::warn!(
            kind,
            start = s,
            configured_end = end,
            "port_range_end < start : clamp a start (range mono-port)"
        );
        e = s;
    }
    (s, e)
}

fn ranges_overlap(left_start: u16, left_end: u16, right_start: u16, right_end: u16) -> bool {
    left_start <= right_end && right_start <= left_end
}

pub async fn load_game_portal_config(
    bot_config: &Arc<dyn BotConfigRepository>,
    guild_id: &str,
) -> Result<GamePortalConfig, DomainError> {
    let entries = bot_config.get_config(guild_id, "game-portal").await?;
    let (port_range_start, port_range_end) = sanitize_port_range(
        "game",
        parse_u16(find(&entries, "port_range_start"), 25500),
        parse_u16(find(&entries, "port_range_end"), 25599),
    );
    let (rcon_port_range_start, rcon_port_range_end) = sanitize_port_range(
        "rcon",
        parse_u16(find(&entries, "rcon_port_range_start"), 25700),
        parse_u16(find(&entries, "rcon_port_range_end"), 25799),
    );
    if ranges_overlap(
        port_range_start,
        port_range_end,
        rcon_port_range_start,
        rcon_port_range_end,
    ) {
        return Err(DomainError::ValidationError(format!(
            "Les plages game ({port_range_start}-{port_range_end}) et RCON ({rcon_port_range_start}-{rcon_port_range_end}) ne doivent pas se chevaucher"
        )));
    }
    Ok(GamePortalConfig {
        enabled: parse_bool(find(&entries, "enabled"), true),
        max_servers_per_guild: parse_i32(find(&entries, "max_servers_per_guild"), 5),
        max_memory_total_mb: parse_i32(find(&entries, "max_memory_total_mb"), 8192),
        port_range_start,
        port_range_end,
        rcon_port_range_start,
        rcon_port_range_end,
        allowed_templates: parse_csv(
            find(&entries, "allowed_templates"),
            "minecraft-vanilla,valheim,terraria,factorio,palworld,ark,7dtd",
        ),
        default_idle_shutdown_days: parse_i32(find(&entries, "default_idle_shutdown_days"), 7),
        docker_network_name: parse_string(find(&entries, "docker_network_name"), "sentinel-games"),
        container_user: parse_string(find(&entries, "container_user"), "1000:1000"),
        host_data_dir: parse_string(find(&entries, "host_data_dir"), "/var/lib/sentinel/games"),
        auto_create_world_volume: parse_bool(find(&entries, "auto_create_world_volume"), true),
        rcon_enabled: parse_bool(find(&entries, "rcon_enabled"), true),
        log_channel_id: find(&entries, "log_channel_id").map(String::from),
        auto_remove_unused_images: parse_bool(find(&entries, "auto_remove_unused_images"), true),
        unused_image_grace_days: parse_i32(find(&entries, "unused_image_grace_days"), 7),
        rcon_timeout_secs: parse_u32(find(&entries, "rcon_timeout_secs"), 5),
        auto_restart_on_crash: parse_bool(find(&entries, "auto_restart_on_crash"), true),
        max_auto_restart_attempts: parse_i32(
            find(&entries, "max_auto_restart_attempts"),
            DEFAULT_MAX_RESTART_ATTEMPTS,
        ),
        restart_backoff_base_secs: parse_i64(
            find(&entries, "restart_backoff_base_secs"),
            DEFAULT_RESTART_BACKOFF_BASE_SECS,
        )
        .max(1),
        restart_backoff_cap_secs: parse_i64(
            find(&entries, "restart_backoff_cap_secs"),
            DEFAULT_RESTART_BACKOFF_CAP_SECS,
        )
        .max(1),
        stuck_transition_threshold_minutes: parse_i64(
            find(&entries, "stuck_transition_threshold_minutes"),
            10,
        )
        .max(1),
        stop_timeout_secs: parse_u32(find(&entries, "stop_timeout_secs"), 30),
        // Borne dure : jamais plus de 5000 lignes en une requete (protege la
        // memoire / le payload, meme si l'admin configure une valeur absurde).
        max_log_lines: parse_u32(find(&entries, "max_log_lines"), 1000).min(5000),
        session_category_id: find(&entries, "session_category_id")
            .filter(|s| !s.is_empty())
            .map(String::from),
        ip_reveal_default_days: parse_i32(find(&entries, "ip_reveal_default_days"), 7)
            .clamp(0, 365),
        session_daily_ping_enabled: parse_bool(find(&entries, "session_daily_ping_enabled"), false),
        session_daily_ping_hour: parse_i32(find(&entries, "session_daily_ping_hour"), 18)
            .clamp(0, 23),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_clamps_privileged_start() {
        // start 80 -> 1024 ; end 2000 reste au-dessus -> conserve.
        let (s, e) = sanitize_port_range("game", 80, 2000);
        assert_eq!(s, MIN_UNPRIVILEGED_PORT);
        assert_eq!(e, 2000);
    }

    #[test]
    fn sanitize_clamps_end_below_start() {
        // start privilegie ramene a 1024, end (500) < start -> ramene a 1024.
        let (s, e) = sanitize_port_range("rcon", 100, 500);
        assert_eq!(s, MIN_UNPRIVILEGED_PORT);
        assert_eq!(e, MIN_UNPRIVILEGED_PORT);
    }

    #[test]
    fn sanitize_keeps_valid_defaults() {
        assert_eq!(sanitize_port_range("game", 25500, 25599), (25500, 25599));
        assert_eq!(sanitize_port_range("rcon", 25700, 25799), (25700, 25799));
    }

    #[test]
    fn sanitize_end_below_start_in_valid_range() {
        // les deux >= 1024 mais end < start -> end ramene a start.
        assert_eq!(sanitize_port_range("game", 26000, 25000), (26000, 26000));
    }

    #[test]
    fn parse_bool_accepts_case_and_common_admin_values() {
        assert!(parse_bool(Some("TRUE"), false));
        assert!(parse_bool(Some(" Yes "), false));
        assert!(!parse_bool(Some("OFF"), true));
        assert!(!parse_bool(Some("NO"), true));
    }

    #[test]
    fn ranges_overlap_detects_shared_boundary_and_keeps_disjoint_ranges() {
        assert!(ranges_overlap(25500, 25599, 25599, 25699));
        assert!(ranges_overlap(25500, 25599, 25550, 25650));
        assert!(!ranges_overlap(25500, 25599, 25600, 25699));
    }
}
