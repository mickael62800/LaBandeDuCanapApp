//! Adapters outbound vers l'host pour le domaine securite (file-shim de bans
//! et lecture du statut fail2ban). Tous lisent/ecrivent des fichiers sous
//! `/var/lib/sentinel/` partages avec des cron host.

pub mod ban_queue;
pub mod fail2ban;
pub mod probe_reader;
pub mod tls_cert;
