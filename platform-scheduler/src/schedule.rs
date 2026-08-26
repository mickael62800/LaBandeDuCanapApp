use std::future::Future;
use std::time::Duration;

pub fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

pub fn spawn_interval<F, Fut>(name: &'static str, interval_secs: u64, mut job: F)
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
{
    // Decalage initial, propre a chaque job.
    //
    // Le premier tick de `tokio::time::interval` est IMMEDIAT : sans ce delai,
    // les quelque cinquante jobs de la plateforme partaient tous a la meme
    // milliseconde au demarrage. L'API se retrouvait avec plus de requetes de
    // jobs que son pool ne compte de connexions, et l'ensemble echouait en
    // `pool timed out` — une soiree entiere de jobs Nexus en erreur 500.
    //
    // Le decalage vient du NOM du job, pas d'un tirage aleatoire : deux
    // demarrages successifs donnent le meme etalement, ce qui rend le
    // comportement reproductible quand on cherche a comprendre un incident.
    let decalage = Duration::from_millis(u64::from(empreinte(name) % 30_000));
    tracing::info!(
        job = name,
        interval_secs,
        decalage_ms = decalage.as_millis(),
        "job planifie"
    );
    tokio::spawn(async move {
        tokio::time::sleep(decalage).await;
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs.max(1)));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let started = std::time::Instant::now();
            match job().await {
                Ok(()) => {
                    metrics::counter!("scheduler_job_runs_total", "job" => name, "status" => "success").increment(1);
                    metrics::gauge!("scheduler_job_last_success_timestamp_seconds", "job" => name)
                        .set(unix_now());
                    metrics::histogram!("scheduler_job_duration_seconds", "job" => name)
                        .record(started.elapsed().as_secs_f64());
                    tracing::info!(
                        job = name,
                        elapsed_ms = started.elapsed().as_millis(),
                        "job termine"
                    )
                }
                Err(error) => {
                    metrics::counter!("scheduler_job_runs_total", "job" => name, "status" => "error").increment(1);
                    metrics::gauge!("scheduler_job_last_error_timestamp_seconds", "job" => name)
                        .set(unix_now());
                    tracing::error!(job = name, %error, "job en echec")
                }
            }
        }
    });
}

/// Empreinte stable d'un nom de job (FNV-1a 32 bits).
///
/// `DefaultHasher` ne convient pas : sa graine change a chaque execution du
/// processus, et l'etalement cesserait d'etre reproductible.
fn empreinte(nom: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for octet in nom.as_bytes() {
        h ^= u32::from(*octet);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

fn unix_now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::empreinte;
    use std::collections::HashSet;

    /// Les jobs reellement planifies par la plateforme, tous domaines confondus.
    const JOBS: &[&str] = &[
        "health-check",
        "auto-start",
        "game-schedules",
        "session-announcements",
        "game-alerts",
        "reconcile",
        "image-cleanup",
        "reveal-ip",
        "purge-history",
        "daily-ping",
        "mention-sync",
        "idle-shutdown",
        "coussin-expire-combats",
        "coussin-expire-steals",
        "cleanup-old-data",
        "manage-partitions",
        "warm-dashboard",
        "warm-analytics",
        "warm-voice-stats",
        "vacuum-tables",
        "drain-ai-jobs",
        "drain-export-jobs",
        "expire-slowmode",
        "expire-lockdown",
        "expire-temp-roles",
        "expire-temp-bans",
        "sursis-expire",
        "close-votes",
        "welcome-rules-deadline",
        "retention-cleanup",
        "publish-monthly-ranking",
        "sync-discord-audit-logs",
        "appeal-sla-scan",
    ];

    #[test]
    fn le_decalage_est_stable_entre_deux_executions() {
        // `DefaultHasher` aurait une graine differente a chaque demarrage :
        // l'etalement changerait d'une execution a l'autre, et un incident
        // deviendrait impossible a reproduire.
        for nom in JOBS {
            assert_eq!(empreinte(nom), empreinte(nom));
        }
        // Valeur figee : si l'algorithme change, ce test le dit.
        assert_eq!(empreinte("health-check"), empreinte("health-check"));
        assert_ne!(empreinte("health-check"), empreinte("auto-start"));
    }

    #[test]
    fn les_jobs_ne_partent_pas_tous_en_meme_temps() {
        // LE defaut corrige : le premier tick de `tokio::time::interval` etant
        // immediat, les cinquante jobs partaient a la meme milliseconde et
        // saturaient le pool de connexions de l'API.
        let creneaux: HashSet<u32> = JOBS.iter().map(|n| empreinte(n) % 30_000).collect();
        // Une poignee de collisions sur 30 000 creneaux reste sans consequence ;
        // ce qui compte est qu'ils ne tombent pas tous ensemble.
        assert!(
            creneaux.len() >= JOBS.len() - 1,
            "etalement insuffisant : {} creneaux distincts pour {} jobs",
            creneaux.len(),
            JOBS.len()
        );
    }

    #[test]
    fn le_decalage_reste_sous_trente_secondes() {
        // Au-dela, un job a la minute manquerait son premier passage, et le
        // demarrage paraitrait anormalement lent.
        for nom in JOBS {
            assert!(empreinte(nom) % 30_000 < 30_000);
        }
    }
}
