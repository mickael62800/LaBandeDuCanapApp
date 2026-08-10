use std::time::Duration;

pub fn start() {
    let api_url = std::env::var("NEXUS_API_URL").unwrap_or_else(|_| "http://localhost:3100".into());
    let api_key = std::env::var("NEXUS_API_KEY").unwrap_or_default();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let mut request = client.post(format!(
                "{api_url}/api/grand-salon/internal/jobs/close-motions"
            ));
            if !api_key.is_empty() {
                request = request.bearer_auth(&api_key);
            }
            match request.send().await {
                Ok(response) if response.status().is_success() => {
                    tracing::info!("grand_salon: motions echues traitees")
                }
                Ok(response) => tracing::warn!(
                    status = %response.status(),
                    "grand_salon: cloture refusee"
                ),
                Err(error) => tracing::warn!(%error, "grand_salon: API indisponible"),
            }
        }
    });
}
