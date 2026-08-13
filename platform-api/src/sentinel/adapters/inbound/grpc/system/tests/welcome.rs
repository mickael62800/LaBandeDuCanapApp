use super::*;
use async_trait::async_trait;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase;
use platform_core::sentinel::ports::inbound::community::manage_welcome_config::WelcomeConfigPatch;
use platform_core::sentinel::ports::outbound::community::welcome_config_repository::WelcomeConfigData;
use std::sync::Arc;
use std::sync::Mutex;

/// Mock du use case (et non plus du repo) — l'adapter gRPC ne doit
/// connaitre que le port inbound (cf. archi hexagonale).
struct MockWelcomeUc {
    config: Mutex<Result<WelcomeConfigData, DomainError>>,
}

impl MockWelcomeUc {
    fn with_config(cfg: WelcomeConfigData) -> Self {
        Self {
            config: Mutex::new(Ok(cfg)),
        }
    }
    fn with_err() -> Self {
        Self {
            config: Mutex::new(Err(DomainError::Internal("pg down".into()))),
        }
    }
}

#[async_trait]
impl ManageWelcomeConfigUseCase for MockWelcomeUc {
    async fn get(&self, _: &str) -> Result<WelcomeConfigData, DomainError> {
        match &*self.config.lock().unwrap() {
            Ok(c) => Ok(c.clone()),
            Err(e) => Err(DomainError::Internal(format!("{e:?}"))),
        }
    }
    async fn save_patch(
        &self,
        _: &str,
        _: WelcomeConfigPatch,
    ) -> Result<WelcomeConfigData, DomainError> {
        unimplemented!()
    }
}

fn sample_config() -> WelcomeConfigData {
    WelcomeConfigData {
        guild_id: "g1".into(),
        welcome_enabled: true,
        welcome_channel_id: Some("c-welcome".into()),
        welcome_message: "Bienvenue!".into(),
        welcome_embed_color: "0x57F287".into(),
        welcome_dm_enabled: false,
        welcome_dm_message: "".into(),
        leave_enabled: true,
        leave_channel_id: Some("c-leave".into()),
        leave_message: "Au revoir".into(),
        rules_enabled: false,
        rules_channel_id: None,
        rules_message: "".into(),
        rules_role_id: None,
        rules_button_label: "Accepter".into(),
        age_check_enabled: false,
        age_minimum: 0,
        unverified_role_id: None,
        age_modal_question: String::new(),
        age_ban_message: String::new(),
        age_min: 5,
        age_max: 120,
        age_ban_days_per_year: 365,
        age_ban_log_channel_id: None,
        leave_embed_color: "e74c3c".into(),
        rules_embed_color: "5865f2".into(),
        counter_enabled: true,
        counter_channel_id: Some("c-counter".into()),
        counter_format: "{count} membres".into(),
        voice_counter_enabled: false,
        voice_counter_channel_id: None,
        voice_counter_format: "En Vocal : {count}".into(),
        anniversary_enabled: false,
        anniversary_channel_id: None,
        anniversary_message: "".into(),
        rejoin_message: "De retour!".into(),
        welcome_title: "".into(),
        welcome_image_url: "".into(),
        welcome_footer_text: "".into(),
        rejoin_title: "".into(),
        rejoin_image_url: "".into(),
        rejoin_footer_text: "".into(),
        leave_title: "".into(),
        leave_image_url: "".into(),
        leave_footer_text: "".into(),
        anniversary_title: "".into(),
        anniversary_image_url: "".into(),
        anniversary_footer_text: "".into(),
    }
}

#[tokio::test]
async fn get_config_maps_all_fields() {
    let grpc = WelcomeGrpc {
        uc: Arc::new(MockWelcomeUc::with_config(sample_config())),
    };
    let resp = grpc
        .get_config(Request::new(proto::GetConfigRequest {
            guild_id: "g1".into(),
        }))
        .await
        .unwrap();
    let inner = resp.into_inner();
    assert_eq!(inner.guild_id, "g1");
    assert!(inner.welcome_enabled);
    assert_eq!(inner.welcome_channel_id.as_deref(), Some("c-welcome"));
    assert_eq!(inner.welcome_message, "Bienvenue!");
    assert_eq!(inner.welcome_embed_color, "0x57F287");
    assert!(!inner.welcome_dm_enabled);
    assert!(inner.leave_enabled);
    assert_eq!(inner.leave_channel_id.as_deref(), Some("c-leave"));
    assert!(!inner.rules_enabled);
    assert_eq!(inner.rules_button_label, "Accepter");
    assert!(inner.counter_enabled);
    assert_eq!(inner.counter_format, "{count} membres");
    assert!(!inner.anniversary_enabled);
    assert_eq!(inner.rejoin_message, "De retour!");
}

#[tokio::test]
async fn get_config_repo_error_maps_to_internal() {
    let grpc = WelcomeGrpc {
        uc: Arc::new(MockWelcomeUc::with_err()),
    };
    let err = grpc
        .get_config(Request::new(proto::GetConfigRequest {
            guild_id: "g".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Internal);
    assert!(err.message().contains("get welcome config"));
}

#[tokio::test]
async fn get_config_preserves_none_optionals() {
    let mut cfg = sample_config();
    cfg.welcome_channel_id = None;
    cfg.leave_channel_id = None;
    cfg.rules_channel_id = None;
    cfg.rules_role_id = None;
    cfg.counter_channel_id = None;
    cfg.anniversary_channel_id = None;
    let grpc = WelcomeGrpc {
        uc: Arc::new(MockWelcomeUc::with_config(cfg)),
    };
    let inner = grpc
        .get_config(Request::new(proto::GetConfigRequest {
            guild_id: "g".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(inner.welcome_channel_id.is_none());
    assert!(inner.leave_channel_id.is_none());
    assert!(inner.rules_channel_id.is_none());
    assert!(inner.rules_role_id.is_none());
    assert!(inner.counter_channel_id.is_none());
    assert!(inner.anniversary_channel_id.is_none());
}
