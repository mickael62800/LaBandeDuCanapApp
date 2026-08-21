use super::*;
use crate::nexus::domain::entities::casino::game_sync::DiscordRole;
use crate::nexus::domain::entities::system::discord_ids::{ChannelId, MessageId};
use crate::nexus::ports::outbound::casino::game_repository::{Game, GamePanel};
use crate::nexus::ports::outbound::casino::game_sync_repository::StoredInventory;
use async_trait::async_trait;
use std::sync::Mutex;

struct MockGameRepo {
    games: Vec<Game>,
    panels: Vec<GamePanel>,
    role_changes: Mutex<Vec<(String, Option<String>)>>,
    panel_deleted: Mutex<Vec<String>>,
}

#[async_trait]
impl GameRepository for MockGameRepo {
    async fn list(&self, _guild_id: &str) -> Result<Vec<Game>, DomainError> {
        Ok(self.games.clone())
    }
    async fn list_by_category(
        &self,
        _guild_id: &str,
        _cat: Option<&str>,
    ) -> Result<Vec<Game>, DomainError> {
        Ok(self.games.clone())
    }
    async fn find_by_name(
        &self,
        _guild_id: &str,
        _name: &str,
    ) -> Result<Option<Game>, DomainError> {
        Ok(None)
    }
    async fn create(
        &self,
        _guild_id: &str,
        _game_name: &str,
        _created_by: &str,
        _emoji: Option<&str>,
        _category: Option<&str>,
        _role_id: Option<&str>,
    ) -> Result<Game, DomainError> {
        Err(DomainError::NotImplemented("test".into()))
    }
    async fn update(
        &self,
        _guild_id: &str,
        _game_id: &str,
        _game_name: Option<&str>,
        _emoji: Option<Option<&str>>,
        _category: Option<Option<&str>>,
    ) -> Result<Option<Game>, DomainError> {
        Err(DomainError::NotImplemented("test".into()))
    }
    async fn delete(&self, _guild_id: &str, _game_id: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn set_role_id(
        &self,
        guild_id: &str,
        game_id: &str,
        role_id: Option<&str>,
    ) -> Result<Option<Game>, DomainError> {
        self.role_changes
            .lock()
            .unwrap()
            .push((game_id.to_string(), role_id.map(String::from)));
        Ok(None)
    }
    async fn list_panels(&self, _guild_id: &str) -> Result<Vec<GamePanel>, DomainError> {
        Ok(self.panels.clone())
    }
    async fn find_panel_by_message(
        &self,
        _guild_id: &str,
        _msg_id: &str,
    ) -> Result<Option<GamePanel>, DomainError> {
        Ok(None)
    }
    async fn save_panel(
        &self,
        _guild_id: &str,
        _channel_id: &str,
        _message_id: &str,
        _category: Option<&str>,
    ) -> Result<GamePanel, DomainError> {
        Err(DomainError::NotImplemented("test".into()))
    }
    async fn delete_panel(&self, _guild_id: &str, message_id: &str) -> Result<bool, DomainError> {
        self.panel_deleted
            .lock()
            .unwrap()
            .push(message_id.to_string());
        Ok(true)
    }
}

struct MockSyncRepo {
    inventory: Mutex<Option<StoredInventory>>,
}

#[async_trait]
impl GameSyncRepository for MockSyncRepo {
    async fn save_inventory(
        &self,
        _guild_id: &str,
        inventory: &DiscordInventory,
    ) -> Result<(), DomainError> {
        *self.inventory.lock().unwrap() = Some(StoredInventory {
            inventory: inventory.clone(),
            taken_at: "2026-01-01T00:00:00Z".into(),
        });
        Ok(())
    }
    async fn latest_inventory(
        &self,
        _guild_id: &str,
    ) -> Result<Option<StoredInventory>, DomainError> {
        Ok(self.inventory.lock().unwrap().clone())
    }
    async fn guilds_with_games(&self) -> Result<Vec<String>, DomainError> {
        Ok(vec![])
    }
}

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

fn sample_game(id: &str, name: &str, role_id: Option<&str>) -> Game {
    use crate::nexus::domain::entities::system::discord_ids::GuildId;
    Game {
        id: id.into(),
        guild_id: GuildId("guild1".into()),
        game_name: name.into(),
        created_by: "user1".into(),
        created_at: "2026-01-01".into(),
        role_id: role_id.map(String::from),
        emoji: None,
        category: None,
    }
}

fn sample_panel(id: &str, chan: &str, msg: &str) -> GamePanel {
    use crate::nexus::domain::entities::system::discord_ids::GuildId;
    GamePanel {
        id: id.into(),
        guild_id: GuildId("guild1".into()),
        channel_id: ChannelId::new(chan),
        message_id: MessageId::new(msg),
        category: None,
    }
}

#[tokio::test]
async fn test_report_and_record_inventory() {
    let game_repo = Arc::new(MockGameRepo {
        games: vec![sample_game("g1", "Minecraft", Some("role1"))],
        panels: vec![sample_panel("p1", "chan1", "msg1")],
        role_changes: Mutex::new(vec![]),
        panel_deleted: Mutex::new(vec![]),
    });
    let sync_repo = Arc::new(MockSyncRepo {
        inventory: Mutex::new(None),
    });
    let events = Arc::new(MockEventPublisher {
        published: Mutex::new(vec![]),
    });

    let service = GameSyncService::new(game_repo, sync_repo.clone(), events);

    // Initial report with no inventory
    let rep = service.report("guild1").await.unwrap();
    assert!(rep.inventory_taken_at.is_none());

    // Record inventory
    let inv = DiscordInventory {
        roles: vec![DiscordRole {
            id: "role1".into(),
            name: "Minecraft".into(),
            color: 0,
            mentionable: true,
        }],
        live_panel_messages: vec!["msg1".into()],
        unreadable_channels: vec![],
    };
    service.record_inventory("guild1", &inv).await.unwrap();

    // Report after inventory recorded
    let rep2 = service.report("guild1").await.unwrap();
    assert!(rep2.inventory_taken_at.is_some());
}

#[tokio::test]
async fn test_request_inventory_publishes_event() {
    let game_repo = Arc::new(MockGameRepo {
        games: vec![],
        panels: vec![],
        role_changes: Mutex::new(vec![]),
        panel_deleted: Mutex::new(vec![]),
    });
    let sync_repo = Arc::new(MockSyncRepo {
        inventory: Mutex::new(None),
    });
    let events = Arc::new(MockEventPublisher {
        published: Mutex::new(vec![]),
    });

    let service = GameSyncService::new(game_repo, sync_repo, events.clone());
    service.request_inventory("guild1").await;

    let pub_events = events.published.lock().unwrap();
    assert_eq!(pub_events.len(), 1);
    assert_eq!(pub_events[0].0, game_events::GAMES_SYNC_REQUESTED);
}

#[tokio::test]
async fn test_forget_vanished_role() {
    let game_repo = Arc::new(MockGameRepo {
        games: vec![sample_game("g1", "Minecraft", Some("role1"))],
        panels: vec![],
        role_changes: Mutex::new(vec![]),
        panel_deleted: Mutex::new(vec![]),
    });
    let sync_repo = Arc::new(MockSyncRepo {
        inventory: Mutex::new(None),
    });
    let events = Arc::new(MockEventPublisher {
        published: Mutex::new(vec![]),
    });

    let service = GameSyncService::new(game_repo.clone(), sync_repo, events);
    let touched = service
        .forget_vanished_role("guild1", "role1")
        .await
        .unwrap();
    assert_eq!(touched, vec!["Minecraft".to_string()]);

    let changes = game_repo.role_changes.lock().unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].0, "g1");
    assert_eq!(changes[0].1, None);
}

#[tokio::test]
async fn test_resolve_divergences() {
    let game_repo = Arc::new(MockGameRepo {
        games: vec![
            sample_game("g1", "Minecraft", Some("role1")),
            sample_game("g2", "Valheim", None), // unbound
        ],
        panels: vec![sample_panel("p1", "chan1", "msg1")],
        role_changes: Mutex::new(vec![]),
        panel_deleted: Mutex::new(vec![]),
    });
    // Simuler un inventaire Discord
    let sync_repo = Arc::new(MockSyncRepo {
        inventory: Mutex::new(Some(StoredInventory {
            inventory: DiscordInventory {
                roles: vec![DiscordRole {
                    id: "orphan_role".into(),
                    name: "Orphan Game".into(),
                    color: 0x3498DB, // DEFAULT_GAME_ROLE_COLOR
                    mentionable: true,
                }],
                live_panel_messages: vec![], // panel missing
                unreadable_channels: vec![],
            },
            taken_at: "2026-01-01T00:00:00Z".into(),
        })),
    });
    let events = Arc::new(MockEventPublisher {
        published: Mutex::new(vec![]),
    });

    let service = GameSyncService::new(game_repo.clone(), sync_repo, events.clone());

    // 1. Écart de rôle manquant sur Discord, résolution Discord (on détache)
    let res = service
        .resolve("guild1", "role_missing:g1", SyncDirection::Discord)
        .await
        .unwrap();
    assert!(res.applied_now);
    assert_eq!(game_repo.role_changes.lock().unwrap()[0].1, None);

    // 2. Résolution Dashboard (on recrée via le bot)
    let res2 = service
        .resolve("guild1", "role_missing:g1", SyncDirection::Dashboard)
        .await
        .unwrap();
    assert!(!res2.applied_now);
    assert!(res2.requested_from_discord);
    assert_eq!(
        events.published.lock().unwrap()[0].0,
        game_events::GAMES_ROLES_ENSURE
    );

    // 3. Écart de rôle non lié, résolution Discord (doit échouer !)
    let err = service
        .resolve("guild1", "role_unbound:g2", SyncDirection::Discord)
        .await;
    assert!(err.is_err());

    // 4. Écart de rôle non lié, résolution Dashboard
    let res3 = service
        .resolve("guild1", "role_unbound:g2", SyncDirection::Dashboard)
        .await
        .unwrap();
    assert!(!res3.applied_now);
    assert!(res3.requested_from_discord);

    // 5. Rôle orphelin, résolution Discord (conserve le rôle)
    let res4 = service
        .resolve("guild1", "role_orphan:orphan_role", SyncDirection::Discord)
        .await
        .unwrap();
    assert!(!res4.applied_now);

    // 6. Rôle orphelin, résolution Dashboard (supprime le rôle)
    let res5 = service
        .resolve(
            "guild1",
            "role_orphan:orphan_role",
            SyncDirection::Dashboard,
        )
        .await
        .unwrap();
    assert!(!res5.applied_now);
    assert_eq!(
        events.published.lock().unwrap()[2].0,
        game_events::GAME_ROLE_DELETE
    );

    // 7. Panneau manquant, résolution Discord (oublie le panneau)
    let res6 = service
        .resolve("guild1", "panel_missing:p1", SyncDirection::Discord)
        .await
        .unwrap();
    assert!(res6.applied_now);
    assert_eq!(game_repo.panel_deleted.lock().unwrap()[0], "msg1");

    // 8. Panneau manquant, résolution Dashboard (redéploie le panneau)
    let res7 = service
        .resolve("guild1", "panel_missing:p1", SyncDirection::Dashboard)
        .await
        .unwrap();
    assert!(res7.applied_now);
    assert_eq!(
        events.published.lock().unwrap()[3].0,
        game_events::GAMES_PANEL_DEPLOY
    );
}
