//! Rate limiter dynamique en memoire : compte les requetes par IP sur une
//! fenetre glissante, declenche un ban auto via le shim ban-apply quand
//! le seuil est depasse. Configure via env :
//!   RATE_LIMIT_THRESHOLD (defaut 200 req/min)
//!   RATE_LIMIT_WINDOW_SECS (defaut 60)
//!   RATE_LIMIT_BAN_DURATION_HOURS (defaut 1, indicatif pour fail2ban)

use std::collections::VecDeque;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Adresse d'un reseau prive / non routable sur Internet. `Ipv4Addr::is_private`
/// existe en stable, son equivalent v6 (`is_unique_local`) non — d'ou ce test
/// explicite sur le prefixe `fc00::/7`.
fn est_privee(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || v4.is_unspecified(),
        IpAddr::V6(v6) => {
            v6.is_unspecified() || (v6.segments()[0] & 0xfe00) == 0xfc00 // fc00::/7
        }
    }
}

use dashmap::DashMap;
use tokio::sync::Mutex;

pub struct RateLimiter {
    pub threshold: usize,
    pub window: Duration,
    counts: DashMap<String, Mutex<VecDeque<Instant>>>,
    /// IPs deja bannies recemment pour eviter de spammer le fichier de ban
    recent_bans: DashMap<String, Instant>,
}

impl RateLimiter {
    pub fn from_env() -> Self {
        let threshold = std::env::var("RATE_LIMIT_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(200);
        let window_secs = std::env::var("RATE_LIMIT_WINDOW_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);
        Self {
            threshold,
            window: Duration::from_secs(window_secs),
            counts: DashMap::new(),
            recent_bans: DashMap::new(),
        }
    }

    /// A appeler dans le middleware pour chaque requete. Retourne true si
    /// l'IP doit etre bannie maintenant (pour declenchement async).
    pub async fn observe(&self, ip: &str) -> bool {
        // `0.0.0.0` = pas de `ConnectInfo` (routeur de test) : compter dessus
        // agregerait toutes les requetes sur un bucket unique.
        if ip.is_empty() || ip == "unknown" || ip == "0.0.0.0" {
            return false;
        }
        // Skip si deja banni dans les 5 dernieres minutes
        if let Some(t) = self.recent_bans.get(ip) {
            if t.elapsed() < Duration::from_secs(300) {
                return false;
            }
        }
        let now = Instant::now();
        let entry = self
            .counts
            .entry(ip.to_string())
            .or_insert_with(|| Mutex::new(VecDeque::new()));
        let mut q = entry.value().lock().await;
        // Purge les entrees hors fenetre
        while let Some(front) = q.front() {
            if now.duration_since(*front) > self.window {
                q.pop_front();
            } else {
                break;
            }
        }
        q.push_back(now);
        if q.len() >= self.threshold {
            self.recent_bans.insert(ip.to_string(), now);
            q.clear();
            return true;
        }
        false
    }

    /// Fichier de demandes de ban, consomme par le cron hote
    /// `sentinel-apply-bans.sh` (cf. `infrastructure/scripts/setup-host-security.sh`).
    ///
    /// Format TSV `ip \t timestamp \t raison`, en append : c'est celui que le
    /// shim sait lire, et il vide le fichier apres application. L'ancienne
    /// version ecrivait un `ban-requests.json` que plus rien ne lisait depuis
    /// l'extraction du domaine `ops` — l'auto-ban etait annonce dans l'interface
    /// et ne bannissait personne.
    const BANS_PENDING_PATH: &str = "/var/lib/sentinel/bans-pending.txt";

    /// Ecrit l'IP dans la file de ban consommee par le shim ban-apply.
    pub async fn trigger_ban(self: &Arc<Self>, ip: String) {
        // Le shim passe cette valeur a `ufw deny from "$IP"`, et la ligne est
        // relue en TSV : une valeur qui n'est pas une IP n'a rien a y faire, et
        // une tabulation ou un saut de ligne y injecterait une seconde entree.
        // L'appelant fournit deja une IP resolue par `client_ip`, mais ce
        // fichier declenche une action sur le pare-feu de l'hote : on revalide.
        let Ok(parsed) = ip.parse::<std::net::IpAddr>() else {
            tracing::warn!(valeur = %ip, "auto-ban ignore : ce n'est pas une adresse IP");
            return;
        };
        // Meme garde que `ops_core::validate_bannable_ip` sur le chemin manuel :
        // un `TRUST_PROXY_HOPS` mal regle fait remonter l'IP du reverse proxy,
        // et `ufw deny` sur le loopback ou le reseau Docker coupe la production
        // entiere pour cause d'exces de trafic legitime.
        if parsed.is_loopback() || est_privee(&parsed) {
            tracing::warn!(
                ip = %parsed,
                "auto-ban ignore : adresse locale/privee (verifier TRUST_PROXY_HOPS)"
            );
            return;
        }

        let reason = format!("rate-limit auto: > {} req/{:?}", self.threshold, self.window);
        let ligne = format!("{parsed}\t{}\t{reason}\n", chrono::Utc::now().to_rfc3339());

        // Append : deux bans concurrents ne peuvent pas s'ecraser, et le shim
        // tronque le fichier lui-meme apres application. Le read-modify-write
        // precedent perdait des entrees et faisait croitre le fichier sans borne.
        let ecriture = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(Self::BANS_PENDING_PATH)
            .and_then(|mut f| std::io::Write::write_all(&mut f, ligne.as_bytes()));

        match ecriture {
            Ok(()) => tracing::warn!(ip = %parsed, "rate-limit auto-ban declenche"),
            Err(e) => tracing::error!(error = %e, ip = %parsed, "auto-ban : ecriture impossible"),
        }
    }
}
