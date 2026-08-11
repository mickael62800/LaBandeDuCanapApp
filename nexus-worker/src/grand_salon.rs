use platform_common_worker::http_job::HttpJobClient;

pub fn start(client: HttpJobClient) {
    platform_common_worker::spawn_interval("grand-salon-close-motions", 60, move || {
        let client = client.clone();
        async move {
            client
                .post_empty("/api/grand-salon/internal/jobs/close-motions")
                .await?;
            tracing::info!("grand_salon: motions echues traitees");
            Ok(())
        }
    });
}
