use crate::config::DomainConfig;

pub fn start(config: DomainConfig) {
    let client = config.client;
    crate::schedule::spawn_interval(
        "ops.dispatch-alerts",
        crate::schedule::env_u64("SECURITY_ALERTS_INTERVAL_SECS", 300),
        move || {
            let client = client.clone();
            async move {
                let report: serde_json::Value =
                    client.post_json("/internal/jobs/dispatch-alerts").await?;
                tracing::info!(%report, "alertes Ops evaluees");
                Ok(())
            }
        },
    );
}
