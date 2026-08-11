//! Rate limit par IP (token bucket en memoire), partage par les deux APIs.
//!
//! Le middleware ne depend que de `RateLimiter` — pas de l'etat applicatif —
//! donc il se monte tel quel dans n'importe quel routeur axum.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::ConnectInfo;
use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use tokio::sync::Mutex;

/// Nombre maximum d'IP suivies simultanement.
///
/// Borne l'empreinte memoire : sans plafond, un attaquant qui fait tourner ses
/// IP source (bon marche en IPv6) fait grossir la table jusqu'a l'OOM.
const MAX_BUCKETS: usize = 50_000;

/// Age au-dela duquel un bucket inactif est purge, en secondes.
const BUCKET_TTL_SECS: u64 = 120;
const CLEANUP_INTERVAL_SECS: u64 = 60;

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<LimiterState>>,
    max_tokens: u64,
    refill_per_sec: u64,
}

struct LimiterState {
    buckets: HashMap<IpAddr, Bucket>,
    last_cleanup: Instant,
}

struct Bucket {
    tokens: u64,
    last_refill: Instant,
}

impl RateLimiter {
    /// `requests_per_sec` est le debit soutenu ; le burst autorise vaut 10x.
    pub fn new(requests_per_sec: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LimiterState {
                buckets: HashMap::new(),
                last_cleanup: Instant::now(),
            })),
            max_tokens: requests_per_sec * 10,
            refill_per_sec: requests_per_sec,
        }
    }

    /// Consomme un jeton pour cette IP. `false` = requete a refuser.
    pub async fn check(&self, ip: IpAddr) -> bool {
        let mut state = self.inner.lock().await;
        let now = Instant::now();

        if now.duration_since(state.last_cleanup).as_secs() >= CLEANUP_INTERVAL_SECS {
            state.buckets.retain(|_, bucket| {
                now.duration_since(bucket.last_refill).as_secs() < BUCKET_TTL_SECS
            });
            state.last_cleanup = now;
        }

        let buckets = &mut state.buckets;

        if !buckets.contains_key(&ip) && buckets.len() >= MAX_BUCKETS {
            // Purge d'urgence avant de refuser : une table pleine de buckets
            // perimes ne doit pas bloquer un client legitime.
            buckets.retain(|_, b| now.duration_since(b.last_refill).as_secs() < BUCKET_TTL_SECS);
            if buckets.len() >= MAX_BUCKETS {
                return false;
            }
        }

        let bucket = buckets.entry(ip).or_insert(Bucket {
            tokens: self.max_tokens,
            last_refill: now,
        });

        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        let refill = (elapsed * self.refill_per_sec as f64) as u64;
        if refill > 0 {
            bucket.tokens = (bucket.tokens + refill).min(self.max_tokens);
            bucket.last_refill = now;
        }

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Purge les buckets inactifs. A appeler periodiquement (~60 s).
    pub async fn cleanup(&self) {
        let mut state = self.inner.lock().await;
        let now = Instant::now();
        state
            .buckets
            .retain(|_, b| now.duration_since(b.last_refill).as_secs() < BUCKET_TTL_SECS);
        state.last_cleanup = now;
    }

    /// Nombre d'IP actuellement suivies (diagnostic et tests).
    pub async fn tracked_ips(&self) -> usize {
        self.inner.lock().await.buckets.len()
    }
}

/// Determine l'IP cliente en tenant compte des reverse proxies.
///
/// `X-Forwarded-For` est de la forme `client, proxy1, proxy2` : chaque proxy
/// AJOUTE a droite l'IP qu'il a vue. Prendre la premiere valeur (a gauche)
/// serait une faille : elle est entierement controlee par le client, qui
/// forgerait une IP differente a chaque requete et annulerait le rate limit.
///
/// On compte donc `TRUST_PROXY_HOPS` positions depuis la DROITE (defaut 1).
/// Cette position-la, le client ne peut pas la falsifier : nos proxies ecrivent
/// apres lui.
pub fn client_ip(request: &Request, fallback: IpAddr) -> IpAddr {
    let hops: usize = std::env::var("TRUST_PROXY_HOPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    if hops > 0 {
        if let Some(xff) = request.headers().get("x-forwarded-for") {
            if let Ok(s) = xff.to_str() {
                let ips: Vec<&str> = s
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                if ips.len() >= hops {
                    if let Ok(ip) = ips[ips.len() - hops].parse::<IpAddr>() {
                        return ip;
                    }
                }
            }
        }
        if let Some(xri) = request.headers().get("x-real-ip") {
            if let Ok(s) = xri.to_str() {
                if let Ok(ip) = s.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }
    // Pas de proxy de confiance, ou aucun en-tete exploitable : l'IP de la
    // socket, non falsifiable.
    fallback
}

/// Middleware axum. Exige `into_make_service_with_connect_info::<SocketAddr>()`
/// cote serveur, faute de quoi `ConnectInfo` echoue et tout est rejete.
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    State(limiter): State<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let ip = client_ip(&request, addr.ip());
    if limiter.check(ip).await {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", "1")],
            "Rate limit exceeded",
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[tokio::test]
    async fn consomme_le_burst_puis_refuse() {
        // 1 req/s => burst de 10 jetons.
        let limiter = RateLimiter::new(1);
        let client = ip("203.0.113.7");
        for _ in 0..10 {
            assert!(limiter.check(client).await);
        }
        assert!(!limiter.check(client).await);
    }

    #[tokio::test]
    async fn les_ip_sont_independantes() {
        let limiter = RateLimiter::new(1);
        for _ in 0..10 {
            assert!(limiter.check(ip("203.0.113.7")).await);
        }
        // Un client sature ne doit pas bloquer les autres.
        assert!(limiter.check(ip("203.0.113.8")).await);
    }

    #[tokio::test]
    async fn cleanup_ne_supprime_pas_les_buckets_frais() {
        let limiter = RateLimiter::new(5);
        assert!(limiter.check(ip("203.0.113.10")).await);
        limiter.cleanup().await;
        assert_eq!(limiter.tracked_ips().await, 1);
    }

    #[tokio::test]
    async fn les_jetons_se_rechargent_avec_le_temps() {
        let rl = RateLimiter::new(100); // burst = 1000
        for _ in 0..999 {
            rl.check(ip("10.0.0.1")).await;
        }
        assert!(rl.check(ip("10.0.0.1")).await);
        // ~150 ms doivent suffire a recharger une dizaine de jetons.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        assert!(rl.check(ip("10.0.0.1")).await);
    }

    fn req(headers: &[(&str, &str)]) -> Request {
        let mut b = Request::builder().uri("/");
        for (k, v) in headers {
            b = b.header(*k, *v);
        }
        b.body(Body::empty()).unwrap()
    }

    /// TOUS les cas de `client_ip` tiennent dans un seul test : ils lisent la
    /// meme variable d'environnement, or les tests d'un meme binaire tournent
    /// en parallele. Separes, ils se voleraient la valeur de
    /// `TRUST_PROXY_HOPS` et echoueraient au hasard.
    #[test]
    fn client_ip_couvre_toutes_les_sources() {
        let socket = ip("127.0.0.1");
        std::env::set_var("TRUST_PROXY_HOPS", "1");

        // Le hop de confiance est le plus a DROITE : les valeurs de gauche
        // sont controlees par le client et ne doivent jamais etre retenues.
        assert_eq!(
            client_ip(
                &req(&[("x-forwarded-for", "1.1.1.1, 2.2.2.2, 3.3.3.3")]),
                socket
            ),
            ip("3.3.3.3")
        );

        // XFF absent -> repli sur X-Real-IP.
        assert_eq!(
            client_ip(&req(&[("x-real-ip", "192.168.1.5")]), socket),
            ip("192.168.1.5")
        );

        // Aucun en-tete -> IP de socket.
        assert_eq!(client_ip(&req(&[]), socket), socket);

        // Le hop de confiance n'est pas parseable : on retombe sur la socket,
        // on ne "remonte" PAS vers la gauche (qui est falsifiable).
        assert_eq!(
            client_ip(&req(&[("x-forwarded-for", "10.0.0.1, not-an-ip")]), socket),
            socket
        );

        // Espaces autour de la valeur.
        assert_eq!(
            client_ip(&req(&[("x-forwarded-for", "  10.0.0.42  ")]), socket),
            ip("10.0.0.42")
        );

        // X-Real-IP invalide -> socket.
        assert_eq!(client_ip(&req(&[("x-real-ip", "garbage")]), socket), socket);

        // XFF prime sur X-Real-IP.
        assert_eq!(
            client_ip(
                &req(&[("x-forwarded-for", "1.2.3.4"), ("x-real-ip", "5.6.7.8")]),
                socket
            ),
            ip("1.2.3.4")
        );

        // Moins de sauts que prevu : en-tete forge ou topologie inattendue.
        std::env::set_var("TRUST_PROXY_HOPS", "2");
        assert_eq!(
            client_ip(&req(&[("x-forwarded-for", "1.1.1.1")]), socket),
            socket
        );

        std::env::set_var("TRUST_PROXY_HOPS", "1");
    }
}
