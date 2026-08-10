//! Exploitation : la MACHINE HOTE, pas Discord.
//!
//! Sondes systeme, logs techniques des services, securite de l'hote (TLS, IP
//! bannies, journal d'admin), conteneurs Docker et regles d'alerte. Ces
//! services concernent autant Nexus et Atrium que Sentinel : ils etaient
//! melanges au domaine system, qui portait aussi les tickets, l'OAuth et le
//! reset de guilde — du metier Discord.

pub mod lookup_geoip_service;
pub mod manage_alert_rules_service;
pub mod manage_ip_bans_service;
pub mod manage_security_audit_service;
pub mod manage_server_events_service;
pub mod manage_system_logs_service;
pub mod read_host_probe_service;
pub mod read_security_logs_service;
pub mod read_tls_cert_service;
