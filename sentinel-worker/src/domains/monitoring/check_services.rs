use std::collections::HashSet;

use redis::AsyncCommands;
use tokio::sync::watch;
use tracing::{info, warn};

use platform_common_worker::{JobMetrics, SupervisedTask};

use super::MonitorConfig;

/// Demarre la boucle de monitoring : toutes les X secondes, verifie
/// quels bots/workers sont en ligne et alerte via l'API quand un service disparait.
pub fn start(
    http: reqwest::Client,
    mut redis: redis::aio::ConnectionManager,
    config: MonitorConfig,
    mut shutdown: watch::Receiver<bool>,
) -> SupervisedTask {
    SupervisedTask::spawn("monitor_services", async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
            config.check_interval_secs.max(1),
        ));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut previous_online: HashSet<String> = HashSet::new();
        let mut first_run = true;
        let mut job_metrics = JobMetrics::new("monitor_services", "monitoring-worker");

        loop {
            tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                    continue;
                }
                _ = interval.tick() => {}
            }

            let started = std::time::Instant::now();
            job_metrics.started();

            // Recuperer tous les services connus
            let known: Vec<String> = match redis.smembers::<_, Vec<String>>("bots:known").await {
                Ok(k) => k,
                Err(e) => {
                    job_metrics.failed(started.elapsed());
                    warn!(error = %e, "Erreur lecture bots:known depuis Redis");
                    continue;
                }
            };

            let mut current_online: HashSet<String> = HashSet::new();

            for name in &known {
                let exists: bool = redis
                    .exists(format!("bot:online:{}", name))
                    .await
                    .unwrap_or(false);
                if exists {
                    current_online.insert(name.clone());
                }
            }

            if first_run {
                // Premier check : on enregistre l'etat sans alerter
                previous_online = current_online;
                first_run = false;
                info!(
                    online = previous_online.len(),
                    total = known.len(),
                    "Etat initial des services"
                );
                job_metrics.succeeded(started.elapsed());
                continue;
            }

            // Detecter les services qui viennent de passer offline
            for name in &previous_online {
                if !current_online.contains(name) {
                    let label = service_label(name);
                    warn!(service = %name, label = label, "Service hors ligne");

                    let mut req = http.post(format!("{}/api/logs", config.api_url)).json(
                        &serde_json::json!({
                            "level": "error",
                            "bot": "sentinel-worker",
                            "server": "",
                            "message": format!("{} hors ligne : {}", label, name),
                            "category": "worker",
                        }),
                    );
                    if !config.api_key.is_empty() {
                        req = req.bearer_auth(&config.api_key);
                    }
                    if let Err(e) = req.send().await {
                        warn!(error = %e, service = %name, "Erreur envoi alerte offline a l'API");
                    }

                    let event = serde_json::json!({
                        "event": "bot_status",
                        "data": {
                            "bot": name,
                            "online": false,
                            "type": if sentinel_core::domain::entities::system::config_parsers::is_worker_service(name) { "worker" } else { "bot" },
                        }
                    });
                    if let Err(e) = platform_common_worker::redis_helpers::xadd_event(
                        &mut redis,
                        &event.to_string(),
                    )
                    .await
                    {
                        warn!(error = %e, "Erreur publication event offline sur Redis");
                    }
                }
            }

            // Detecter les services qui viennent de revenir en ligne
            for name in &current_online {
                if !previous_online.contains(name) {
                    let label = service_label(name);
                    info!(service = %name, label = label, "Service en ligne");

                    let mut req = http.post(format!("{}/api/logs", config.api_url)).json(
                        &serde_json::json!({
                            "level": "info",
                            "bot": "sentinel-worker",
                            "server": "",
                            "message": format!("{} en ligne : {}", label, name),
                            "category": "worker",
                        }),
                    );
                    if !config.api_key.is_empty() {
                        req = req.bearer_auth(&config.api_key);
                    }
                    if let Err(e) = req.send().await {
                        warn!(error = %e, service = %name, "Erreur envoi alerte online a l'API");
                    }

                    let event = serde_json::json!({
                        "event": "bot_status",
                        "data": {
                            "bot": name,
                            "online": true,
                            "type": if sentinel_core::domain::entities::system::config_parsers::is_worker_service(name) { "worker" } else { "bot" },
                        }
                    });
                    if let Err(e) = platform_common_worker::redis_helpers::xadd_event(
                        &mut redis,
                        &event.to_string(),
                    )
                    .await
                    {
                        warn!(error = %e, "Erreur publication event online sur Redis");
                    }
                }
            }

            previous_online = current_online;
            job_metrics.succeeded(started.elapsed());
        }

        job_metrics.stopped();
        info!("Boucle de monitoring arretee (shutdown)");
    })
}

/// Retourne le label d'un service (Bot ou Worker). Le prédicat de
/// classification vit dans le core (`is_worker_service`), partagé avec le
/// dashboard de l'API.
fn service_label(name: &str) -> &'static str {
    if sentinel_core::domain::entities::system::config_parsers::is_worker_service(name) {
        "Worker"
    } else {
        "Bot"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_label_bot() {
        assert_eq!(service_label("automod-bot"), "Bot");
        assert_eq!(service_label("image-bot"), "Bot");
    }

    #[test]
    fn service_label_worker() {
        assert_eq!(service_label("moderation-worker"), "Worker");
        assert_eq!(service_label("analytics-worker"), "Worker");
        assert_eq!(service_label("monitoring-worker"), "Worker");
    }

    #[test]
    fn service_label_unknown() {
        // Pas de "worker" dans le nom → default "Bot"
        assert_eq!(service_label("custom-service"), "Bot");
    }
}
