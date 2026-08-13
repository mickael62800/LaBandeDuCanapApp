use super::*;
use async_trait::async_trait;
use chrono::TimeZone;
use platform_core::sentinel::domain::entities::system::bot_config::BotDefinition;
use platform_core::sentinel::domain::entities::system::bot_config::BotGuildConfig;
use platform_core::sentinel::domain::errors::DomainError;
use std::sync::Mutex;
use uuid::Uuid;

/// Config bot no-op : `get_config` renvoie vide (=> delai par defaut 7 jours).
struct NoopConfig;

#[async_trait]
impl BotConfigRepository for NoopConfig {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
        Ok(vec![])
    }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

#[derive(Default)]
struct MockSursisUc {
    created_days: Mutex<Vec<i64>>,
    store: Mutex<Option<Sursis>>,
    resolve_claimed: bool,
    resolved: Mutex<Vec<(Uuid, SursisStatus)>>,
    missing: bool,
}

fn sample_sursis() -> Sursis {
    Sursis {
        id: Uuid::nil(),
        guild_id: "g1".into(),
        user_id: "u1".into(),
        username: "Alice".into(),
        reason: "spam".into(),
        saved_roles: vec!["r1".into(), "r2".into()],
        channel_id: Some("c1".into()),
        status: SursisStatus::from_str_lossy("en_sursis").unwrap(),
        expires_at: chrono::Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap(),
    }
}

#[async_trait]
impl ManageSursisUseCase for MockSursisUc {
    async fn create(&self, cmd: CreateSursisCommand) -> Result<Sursis, DomainError> {
        self.created_days.lock().unwrap().push(cmd.days);
        let mut s = sample_sursis();
        s.user_id = cmd.user_id;
        s.saved_roles = cmd.saved_roles;
        Ok(s)
    }
    async fn get(&self, _id: Uuid) -> Result<Option<Sursis>, DomainError> {
        if self.missing {
            return Ok(None);
        }
        Ok(Some(
            self.store
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(sample_sursis),
        ))
    }
    async fn resolve(&self, id: Uuid, status: SursisStatus) -> Result<bool, DomainError> {
        self.resolved.lock().unwrap().push((id, status));
        Ok(self.resolve_claimed)
    }
    async fn list_due(&self) -> Result<Vec<Sursis>, DomainError> {
        unimplemented!()
    }
}

fn grpc(uc: Arc<MockSursisUc>) -> SursisGrpc {
    SursisGrpc {
        sursis_uc: uc,
        bot_config_repo: Arc::new(NoopConfig),
    }
}

#[tokio::test]
async fn create_uses_default_delay_and_maps_fields() {
    let uc = Arc::new(MockSursisUc::default());
    let resp = grpc(uc.clone())
        .create_sursis(Request::new(proto::CreateSursisRequest {
            guild_id: "g1".into(),
            user_id: "u42".into(),
            username: "Bob".into(),
            moderator_id: "m1".into(),
            moderator_name: "Mod".into(),
            reason: "flood".into(),
            saved_roles: vec!["r9".into()],
            channel_id: Some("c9".into()),
        }))
        .await
        .unwrap()
        .into_inner();
    // Config vide -> delai par defaut 7 jours.
    assert_eq!(uc.created_days.lock().unwrap().as_slice(), &[7]);
    assert_eq!(resp.user_id, "u42");
    assert_eq!(resp.saved_roles, vec!["r9".to_string()]);
    assert!(!resp.expires_at.is_empty());
}

#[tokio::test]
async fn get_maps_fields() {
    let resp = grpc(Arc::new(MockSursisUc::default()))
        .get_sursis(Request::new(proto::GetSursisRequest {
            id: Uuid::nil().to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp.user_id, "u1");
    assert_eq!(resp.saved_roles.len(), 2);
    assert_eq!(resp.channel_id.as_deref(), Some("c1"));
    assert_eq!(resp.status, "en_sursis");
}

#[tokio::test]
async fn get_missing_is_not_found() {
    let uc = Arc::new(MockSursisUc {
        missing: true,
        ..Default::default()
    });
    let err = grpc(uc)
        .get_sursis(Request::new(proto::GetSursisRequest {
            id: Uuid::nil().to_string(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn resolve_returns_claimed_and_forwards_status() {
    let uc = Arc::new(MockSursisUc {
        resolve_claimed: true,
        ..Default::default()
    });
    let resp = grpc(uc.clone())
        .resolve_sursis(Request::new(proto::ResolveSursisRequest {
            id: Uuid::nil().to_string(),
            status: "gracie".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(resp.claimed);
    assert_eq!(uc.resolved.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn resolve_rejects_bad_status() {
    let err = grpc(Arc::new(MockSursisUc::default()))
        .resolve_sursis(Request::new(proto::ResolveSursisRequest {
            id: Uuid::nil().to_string(),
            status: "n_importe_quoi".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn resolve_missing_is_not_found() {
    let uc = Arc::new(MockSursisUc {
        missing: true,
        ..Default::default()
    });
    let err = grpc(uc)
        .resolve_sursis(Request::new(proto::ResolveSursisRequest {
            id: Uuid::nil().to_string(),
            status: "banni".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);
}
