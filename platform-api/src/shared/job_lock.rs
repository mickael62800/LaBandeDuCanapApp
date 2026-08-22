//! Verrou d'exclusion des jobs planifies.
//!
//! ATTENTION AU COUPLAGE. Le verrou est un `pg_advisory_lock`, dont la portee
//! est la SESSION : il faut donc retenir la connexion tant que le job tourne.
//! Le handler, lui, puise ses propres connexions dans le MEME pool. Chaque job
//! en consomme donc au moins deux a la fois.
//!
//! Sans limite, cela s'auto-bloque : le planificateur declenche ses quatorze
//! jobs Nexus au meme instant (le premier tick de `tokio::time::interval` est
//! immediat), quatorze connexions partent en verrous, et les handlers se
//! disputent les six restantes sur un pool de vingt. Ceux qui n'en obtiennent
//! pas attendent trente secondes puis echouent en `pool timed out`, sans que
//! les verrous ne se liberent — ils attendent justement la fin des handlers.
//!
//! Ce n'est pas theorique : la plateforme a passe une soiree dans cet etat,
//! tous les jobs Nexus en erreur 500 onze minutes apres le demarrage. Le
//! correctif d'alors avait releve le pool de 5 a 20, ce qui deplacait le seuil
//! sans supprimer le couplage.
//!
//! D'ou le semaphore ci-dessous : il borne le nombre de jobs simultanes de
//! sorte que `2 x permis` reste sous la taille du plus petit pool.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::future::Future;
use std::sync::OnceLock;
use tokio::sync::Semaphore;

/// Jobs autorises a tourner en meme temps, tous domaines confondus.
///
/// Chacun retient une connexion pour son verrou PLUS celles de son handler :
/// le produit `2 x permis` doit rester sous le plus petit pool applicatif
/// (vingt, cote Sentinel comme cote Nexus), en laissant de la marge au trafic
/// HTTP ordinaire, qui partage ce meme pool.
const JOBS_SIMULTANES_DEFAUT: usize = 8;

fn semaphore() -> &'static Semaphore {
    static S: OnceLock<Semaphore> = OnceLock::new();
    S.get_or_init(|| {
        let permis = std::env::var("JOB_MAX_CONCURRENT")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|n| *n >= 1)
            .unwrap_or(JOBS_SIMULTANES_DEFAUT);
        tracing::info!(permis, "verrou de jobs : concurrence bornee");
        Semaphore::new(permis)
    })
}

pub async fn run<T, F, Fut>(
    pool: &sqlx::PgPool,
    job: &str,
    operation: F,
) -> Result<Option<T>, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let mut connection = pool
        .acquire()
        .await
        .map_err(|error| format!("job lock acquire: {error}"))?;
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtext($1))")
        .bind(job)
        .fetch_one(&mut *connection)
        .await
        .map_err(|error| format!("job lock: {error}"))?;
    if !acquired {
        return Ok(None);
    }
    let result = operation().await;
    if let Err(error) = sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock(hashtext($1))")
        .bind(job)
        .fetch_one(&mut *connection)
        .await
    {
        tracing::error!(job, %error, "liberation du verrou impossible");
    }
    result.map(Some)
}

pub async fn middleware(
    State(pool): State<sqlx::PgPool>,
    request: Request,
    next: Next,
) -> Response {
    let Some(job) = request
        .headers()
        .get("x-scheduler-job")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
    else {
        return next.run(request).await;
    };
    // Place limitee AVANT de toucher au pool. Sans permis, on repond 202 sans
    // rien consommer : le planificateur repassera au tick suivant, et un job
    // idempotent ne perd rien a etre differe. Attendre ici serait pire — la
    // requete occuperait une place pendant que les autres saturent le pool.
    let Ok(_place) = semaphore().try_acquire() else {
        return (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({"job": job, "differe": true})),
        )
            .into_response();
    };

    let mut connection = match pool.acquire().await {
        Ok(c) => c,
        Err(error) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": error.to_string()})),
            )
                .into_response()
        }
    };
    let acquired = sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock(hashtext($1))")
        .bind(&job)
        .fetch_one(&mut *connection)
        .await
        .unwrap_or(false);
    if !acquired {
        return (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({"job": job, "locked": true})),
        )
            .into_response();
    }
    let response = next.run(request).await;
    let _ = sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock(hashtext($1))")
        .bind(&job)
        .fetch_one(&mut *connection)
        .await;
    response
}

#[cfg(test)]
// Ces deux tests comparent des CONSTANTES, ce que clippy signale comme une
// assertion a valeur connue. C'est precisement l'intention : figer un rapport
// entre deux reglages que rien d'autre ne relie, pour qu'on ne puisse pas
// relever l'un en oubliant l'autre. Un message d'echec redige vaut mieux ici
// qu'une erreur de debordement arithmetique.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::JOBS_SIMULTANES_DEFAUT;

    /// Taille par defaut des pools applicatifs les plus petits
    /// (`sentinel/bootstrap/connections.rs` et `NEXUS_DB_MAX_CONNECTIONS`).
    const POOL_LE_PLUS_PETIT: usize = 20;

    #[test]
    fn deux_connexions_par_job_tiennent_dans_le_pool() {
        // L'INVARIANT de ce module. Chaque job retient une connexion pour son
        // verrou d'avance PLUS celles de son handler. Si `2 x permis` depasse le
        // pool, les verrous et les handlers se disputent les memes connexions et
        // l'ensemble se bloque — c'est exactement l'incident qui a motive ce
        // semaphore.
        assert!(
            JOBS_SIMULTANES_DEFAUT * 2 <= POOL_LE_PLUS_PETIT,
            "{} jobs x 2 connexions depassent un pool de {}",
            JOBS_SIMULTANES_DEFAUT,
            POOL_LE_PLUS_PETIT
        );
    }

    #[test]
    fn il_reste_de_la_place_pour_le_trafic_ordinaire() {
        // Les jobs partagent ce pool avec les requetes du tableau de bord. Les
        // laisser le remplir entierement ferait tomber l'interface a chaque
        // vague de jobs.
        // `saturating_sub` et non `-` : une soustraction litterale qui deborde
        // est refusee a la COMPILATION, et le mainteneur qui releve la constante
        // lirait « arithmetic operation will overflow » au lieu de la phrase qui
        // lui dit ce qu'il vient de casser.
        let reserve = POOL_LE_PLUS_PETIT.saturating_sub(JOBS_SIMULTANES_DEFAUT * 2);
        assert!(
            reserve >= 4,
            "seulement {reserve} connexions laissees au trafic HTTP"
        );
    }
}
