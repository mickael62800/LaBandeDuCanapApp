//! Adapter postgres du port `SecurityLogRepository` : agregations sur la table
//! `logs` (categorie `api`). Le mapping fenetre -> intervalle SQL vit ici.
//!
//! Les valeurs interpolees dans le SQL (`interval`, `limit`, `bucket`)
//! proviennent d'un enum du domaine et d'entiers `i64` bornes par le use case
//! (`ReadSecurityLogsService::borne`) : pas d'injection possible, et pas de
//! division par zero sur le bucket.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::pg_err_ctx;
use platform_core::ops::domain::entities::security_log::{
    AuthFailure, LogWindow, TopIp, TrafficPoint,
};
use platform_core::ops::domain::errors::DomainError;
use platform_core::ops::ports::outbound::security_log_repository::SecurityLogRepository;

const TBL: &str = "logs";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

fn interval(window: LogWindow) -> &'static str {
    match window {
        LogWindow::OneHour => "1 hour",
        LogWindow::TwentyFourHours => "24 hours",
        LogWindow::SevenDays => "7 days",
    }
}

pub struct PgSecurityLogRepository {
    pool: PgPool,
}

/// Ecarte les adresses internes des vues de securite.
///
/// Ces ecrans parlent de VISITEURS. Or les appels de service a service passent
/// par le reseau Docker : le planificateur qui interroge l'API apparaissait donc
/// en tete du classement des IP, avec 100 % d'erreurs les soirs ou ses jobs
/// echouaient. Cela noyait les adresses reelles sous du trafic interne, et
/// proposait de bannir un conteneur — ce que `validate_bannable_ip` refuse de
/// toute facon, rendant l'action visible mais inoperante.
///
/// Le `CASE` n'est pas une coquetterie : PostgreSQL peut reordonner les
/// conditions d'un `WHERE`, et une conversion `::inet` sur une valeur qui n'est
/// pas une adresse leve une erreur. `CASE` garantit l'ordre d'evaluation — on ne
/// convertit qu'apres avoir verifie la forme.
const EXCLURE_IP_INTERNES: &str = "\
    AND CASE \
          WHEN details->>'client_ip' ~ '^([0-9]{1,3}[.]){3}[0-9]{1,3}$' \
          THEN NOT ((details->>'client_ip')::inet <<= ANY (ARRAY[ \
                 '10.0.0.0/8', '172.16.0.0/12', '192.168.0.0/16', \
                 '127.0.0.0/8', '169.254.0.0/16' \
               ]::inet[])) \
          ELSE true \
        END ";

impl PgSecurityLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SecurityLogRepository for PgSecurityLogRepository {
    async fn top_ips(&self, window: LogWindow, limit: i64) -> Result<Vec<TopIp>, DomainError> {
        let interval = interval(window);
        let sql = format!(
            "SELECT \
                COALESCE(details->>'client_ip', '-') AS ip, \
                COUNT(*)::bigint AS total, \
                SUM(CASE WHEN level IN ('warn', 'error') THEN 1 ELSE 0 END)::bigint AS failed, \
                MAX(timestamp) AS last_seen \
             FROM logs \
             WHERE category = 'api' \
               AND timestamp > NOW() - INTERVAL '{interval}' \
               AND details->>'client_ip' IS NOT NULL \
               AND details->>'client_ip' != '-' \
             {EXCLURE_IP_INTERNES} \
             GROUP BY ip \
             ORDER BY total DESC \
             LIMIT {limit}"
        );
        let rows = sqlx::query_as::<_, (String, i64, i64, DateTime<Utc>)>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|(client_ip, total, failed, last_seen)| TopIp {
                client_ip,
                total,
                failed,
                last_seen,
            })
            .collect())
    }

    async fn auth_failures(
        &self,
        window: LogWindow,
        limit: i64,
    ) -> Result<Vec<AuthFailure>, DomainError> {
        let interval = interval(window);
        let sql = format!(
            "SELECT \
                timestamp, \
                COALESCE((details->>'status_code')::bigint, 0) AS status, \
                COALESCE(details->>'method', '?') AS method, \
                COALESCE(details->>'route', '?') AS route, \
                COALESCE(details->>'client_ip', '-') AS ip, \
                COALESCE(details->>'user_agent', '') AS ua \
             FROM logs \
             WHERE category = 'api' \
               AND timestamp > NOW() - INTERVAL '{interval}' \
               AND (details->>'status_code')::int IN (401, 403) \
             ORDER BY timestamp DESC \
             LIMIT {limit}"
        );
        let rows = sqlx::query_as::<_, (DateTime<Utc>, i64, String, String, String, String)>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(
                |(timestamp, status_code, method, route, client_ip, user_agent)| AuthFailure {
                    timestamp,
                    status_code,
                    method,
                    route,
                    client_ip,
                    user_agent,
                },
            )
            .collect())
    }

    async fn traffic_points(
        &self,
        window: LogWindow,
        bucket_minutes: i64,
    ) -> Result<Vec<TrafficPoint>, DomainError> {
        let interval = interval(window);
        let sql = format!(
            "SELECT \
                date_trunc('hour', timestamp) + \
                    INTERVAL '{bucket_minutes} min' * \
                    FLOOR(EXTRACT(MINUTE FROM timestamp) / {bucket_minutes}) AS bucket, \
                COUNT(*)::bigint AS total, \
                SUM(CASE WHEN level IN ('warn', 'error') THEN 1 ELSE 0 END)::bigint AS errors \
             FROM logs \
             WHERE category = 'api' \
               AND timestamp > NOW() - INTERVAL '{interval}' \
             GROUP BY bucket \
             ORDER BY bucket ASC"
        );
        let rows = sqlx::query_as::<_, (DateTime<Utc>, i64, i64)>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|(timestamp, total, errors)| TrafficPoint {
                timestamp,
                total,
                errors,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::EXCLURE_IP_INTERNES;

    /// Ce que ce test prouve, et ce qu'il ne prouve pas.
    ///
    /// Il fige le CONTENU du fragment SQL, pas son effet : verifier qu'une IN
    /// interne est bien ecartee demanderait une base. Il attrape donc la
    /// suppression accidentelle d'une plage, pas une erreur de logique SQL.
    #[test]
    fn le_filtre_couvre_les_plages_privees_et_le_reseau_docker() {
        // 172.16.0.0/12 est la plage des reseaux Docker par defaut : c'est
        // elle qui faisait apparaitre le planificateur en tete du classement.
        for plage in [
            "10.0.0.0/8",
            "172.16.0.0/12",
            "192.168.0.0/16",
            "127.0.0.0/8",
            "169.254.0.0/16",
        ] {
            assert!(
                EXCLURE_IP_INTERNES.contains(plage),
                "plage {plage} absente du filtre"
            );
        }
    }

    #[test]
    fn la_conversion_inet_est_gardee_par_un_case() {
        // Sans `CASE`, PostgreSQL peut evaluer le `::inet` avant le controle de
        // forme, et une valeur qui n'est pas une adresse fait echouer TOUTE la
        // requete — le panneau se viderait au lieu de se nettoyer.
        let case = EXCLURE_IP_INTERNES
            .find("CASE")
            .expect("garde CASE absente");
        let cast = EXCLURE_IP_INTERNES
            .find("::inet")
            .expect("conversion absente");
        assert!(case < cast, "la conversion doit etre a l'interieur du CASE");
        assert!(EXCLURE_IP_INTERNES.contains("ELSE true"));
    }
}
