use async_trait::async_trait;
use chrono::Utc;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::nexus::application::economy_config::EmptyBotConfigRepository;
use crate::nexus::application::game::manage_templates_service::ManageGameTemplatesService;
use crate::nexus::domain::entities::game::template::{GameTemplate, PortProtocol};
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::game::manage_game_templates::ManageGameTemplatesUseCase;
use crate::nexus::ports::outbound::game::game_template_repository::GameTemplateRepository;

#[derive(Default)]
struct MockTemplateRepo {
    templates: Mutex<Vec<GameTemplate>>,
}

#[async_trait]
impl GameTemplateRepository for MockTemplateRepo {
    async fn list(&self) -> Result<Vec<GameTemplate>, DomainError> {
        Ok(self.templates.lock().unwrap().clone())
    }
    async fn find_by_id(&self, id: Uuid) -> Result<Option<GameTemplate>, DomainError> {
        let list = self.templates.lock().unwrap();
        Ok(list.iter().find(|t| t.id == id).cloned())
    }
    async fn find_by_slug(&self, slug: &str) -> Result<Option<GameTemplate>, DomainError> {
        let list = self.templates.lock().unwrap();
        Ok(list.iter().find(|t| t.slug == slug).cloned())
    }
}

fn sample_template(slug: &str, name: &str) -> GameTemplate {
    GameTemplate {
        id: Uuid::new_v4(),
        slug: slug.into(),
        name: name.into(),
        description: Some("Test template".into()),
        image: "repo/img:latest".into(),
        category: Some("fps".into()),
        icon: None,
        accent_color: None,
        cover_image_url: None,
        container_port: 25565,
        port_protocol: PortProtocol::Tcp,
        volume_path: "/data".into(),
        run_as_root: false,
        default_memory_mb: 2048,
        min_memory_mb: 1024,
        max_memory_mb: 4096,
        default_env: serde_json::json!({}),
        config_schema: vec![],
        command_schema: vec![],
        supports_rcon: false,
        supports_mods: false,
        idle_shutdown_days: 7,
        init_files: vec![],
        command: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[tokio::test]
async fn test_get_template_by_slug_not_found() {
    let repo = Arc::new(MockTemplateRepo::default());
    let service = ManageGameTemplatesService::new(repo, Arc::new(EmptyBotConfigRepository));

    let res = service.get_by_slug("minecraft").await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_get_template_by_slug_success() {
    let repo = Arc::new(MockTemplateRepo::default());
    let t = sample_template("minecraft", "Minecraft");
    repo.templates.lock().unwrap().push(t.clone());

    let service = ManageGameTemplatesService::new(repo, Arc::new(EmptyBotConfigRepository));

    let found = service.get_by_slug("minecraft").await.unwrap();
    assert_eq!(found.name, "Minecraft");
    assert_eq!(found.id, t.id);
}
