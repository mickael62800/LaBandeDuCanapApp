use crate::ports::outbound::events::{game_events, EventPublisher};
use std::sync::Arc;

pub struct DeployGamesPanelUseCase {
    pub events: Arc<dyn EventPublisher>,
}

impl DeployGamesPanelUseCase {
    pub fn new(events: Arc<dyn EventPublisher>) -> Self {
        Self { events }
    }

    /// Publie GAMES_PANEL_DEPLOY sur `nexus:events` ; c'est `nexus-bot` qui
    /// deploie le panneau. (Le panneau venait de sentinel-bot avant le portage
    /// des jeux ; plus aucun composant Sentinel n'est implique.)
    pub async fn execute(&self, guild_id: &str, channel_id: &str, category: Option<&str>) {
        let payload = serde_json::json!({
            "guild_id": guild_id,
            "channel_id": channel_id,
            "category": category,
        });
        self.events
            .publish(game_events::GAMES_PANEL_DEPLOY, payload)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockEventPublisher {
        published: Mutex<Vec<(String, serde_json::Value)>>,
    }

    #[async_trait]
    impl EventPublisher for MockEventPublisher {
        async fn publish(&self, event: &str, data: serde_json::Value) {
            self.published
                .lock()
                .unwrap()
                .push((event.to_string(), data));
        }
    }

    #[tokio::test]
    async fn test_deploy_games_panel_publishes_event() {
        let publisher = Arc::new(MockEventPublisher::default());
        let uc = DeployGamesPanelUseCase::new(publisher.clone());

        uc.execute("guild123", "channel456", Some("fps")).await;

        let events = publisher.published.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, game_events::GAMES_PANEL_DEPLOY);
        assert_eq!(events[0].1["guild_id"], "guild123");
        assert_eq!(events[0].1["channel_id"], "channel456");
        assert_eq!(events[0].1["category"], "fps");
    }
}
