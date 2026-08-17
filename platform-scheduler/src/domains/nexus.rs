use std::time::Duration;

use serde::Deserialize;

use crate::config::DomainConfig;

#[derive(Deserialize, Default)]
struct JobReport {
    #[serde(default)]
    processed: usize,
    #[serde(default)]
    errors: usize,
}

pub fn start(config: DomainConfig) {
    for (job, interval) in [
        ("health-check", env("GAME_HEALTH_CHECK_INTERVAL_SECS", 30)),
        (
            "idle-shutdown",
            env("GAME_IDLE_SHUTDOWN_CHECK_INTERVAL_SECS", 3_600),
        ),
        ("reconcile", env("GAME_RECONCILER_INTERVAL_SECS", 3_600)),
        (
            "image-cleanup",
            env("GAME_IMAGE_CLEANUP_INTERVAL_SECS", 86_400),
        ),
        ("reveal-ip", env("GAME_REVEAL_IP_INTERVAL_SECS", 300)),
        ("daily-ping", env("GAME_DAILY_PING_INTERVAL_SECS", 3_600)),
        ("auto-start", env("GAME_AUTO_START_INTERVAL_SECS", 60)),
        // Verification des jeux mentionnables : demande aux guildes leur
        // inventaire Discord, pour que les divergences se voient sans attendre
        // qu'un humain les soupconne. Espacee : une desynchronisation n'est pas
        // une urgence, et chaque passage fait travailler le bot.
        (
            "mention-sync",
            env("GAME_MENTION_SYNC_INTERVAL_SECS", 21_600),
        ),
        // Defis de Coussin Piege laisses sans reponse : ils bloquent le tour
        // de l'attaquant tant qu'ils trainent. Toutes les 15 min, ce qui borne
        // le blocage residuel bien en dessous de l'echeance elle-meme (24 h).
        (
            "coussin-expire-combats",
            env("COUSSIN_EXPIRE_COMBATS_INTERVAL_SECS", 900),
        ),
        // Fouilles dont la fenetre de defense s'est fermee. Passage court :
        // la fenetre dure une minute, un denouement qui arrive dix minutes
        // plus tard ne raconte plus rien a personne.
        (
            "coussin-expire-steals",
            env("COUSSIN_EXPIRE_STEALS_INTERVAL_SECS", 20),
        ),
    ] {
        let client = config.client.clone();
        crate::schedule::spawn_interval(job_name(job), interval, move || {
            let client = client.clone();
            async move {
                let path = format!("/api/games/internal/jobs/{job}");
                let report: JobReport = client
                    .post_json_with_timeout(&path, Duration::from_secs(240))
                    .await?;
                tracing::info!(
                    job,
                    processed = report.processed,
                    errors = report.errors,
                    "job Nexus execute"
                );
                Ok(())
            }
        });
    }

    let client = config.client;
    crate::schedule::spawn_interval("nexus.close-motions", 60, move || {
        let client = client.clone();
        async move {
            client
                .post_empty("/api/grand-salon/internal/jobs/close-motions")
                .await
        }
    });
}

fn env(name: &str, default: u64) -> u64 {
    crate::schedule::env_u64(name, default)
}

fn job_name(job: &str) -> &'static str {
    match job {
        "health-check" => "nexus.health-check",
        "idle-shutdown" => "nexus.idle-shutdown",
        "reconcile" => "nexus.reconcile",
        "image-cleanup" => "nexus.image-cleanup",
        "reveal-ip" => "nexus.reveal-ip",
        "daily-ping" => "nexus.daily-ping",
        "auto-start" => "nexus.auto-start",
        "mention-sync" => "nexus.mention-sync",
        "coussin-expire-combats" => "nexus.coussin-expire-combats",
        "coussin-expire-steals" => "nexus.coussin-expire-steals",
        _ => "nexus.unknown",
    }
}
