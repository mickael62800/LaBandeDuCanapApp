//! Services d'exploitation.

/// Plafond standard d'une page de listing (endpoints web).
///
/// Duplique volontairement sentinel_core::application::validation : deux
/// constantes ne justifient pas une dependance d'ops-core vers Sentinel, ce
/// qui inverserait le sens des dependances.
pub const PAGE_LIMIT_MAX: i64 = 500;
/// Plafond des listings « batch » (exports, agregations).
pub const BATCH_LIMIT_MAX: i64 = 1000;
pub mod lookup_geoip_service;
pub mod manage_alert_rules_service;
pub mod manage_ip_bans_service;
pub mod manage_security_audit_service;
pub mod manage_server_events_service;
pub mod manage_system_logs_service;
pub mod read_host_probe_service;
pub mod read_security_logs_service;
pub mod read_tls_cert_service;
