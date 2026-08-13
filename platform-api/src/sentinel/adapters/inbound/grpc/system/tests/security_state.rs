use super::*;
use async_trait::async_trait;
use platform_core::sentinel::domain::entities::system::quarantine::ActiveQuarantine;
use platform_core::sentinel::domain::errors::DomainError;
use std::sync::Arc;
use std::sync::Mutex;

/// Mock quarantaine : enregistre les appels pour verifier le mapping.
#[derive(Default)]
struct MockQuarantineUc {
    marked: Mutex<Vec<(String, String, i64)>>,
    lifted: Mutex<Vec<(String, String)>>,
    active: Mutex<Vec<ActiveQuarantine>>,
    fail: bool,
}

#[async_trait]
impl ManageQuarantineUseCase for MockQuarantineUc {
    async fn quarantine_user(
        &self,
        guild_id: &str,
        user_id: &str,
        timeout_secs: i64,
    ) -> Result<(), DomainError> {
        if self.fail {
            return Err(DomainError::Internal("pg down".into()));
        }
        self.marked
            .lock()
            .unwrap()
            .push((guild_id.into(), user_id.into(), timeout_secs));
        Ok(())
    }
    async fn list_active(&self) -> Result<Vec<ActiveQuarantine>, DomainError> {
        Ok(self.active.lock().unwrap().clone())
    }
    async fn lift(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.lifted
            .lock()
            .unwrap()
            .push((guild_id.into(), user_id.into()));
        Ok(())
    }
}

#[derive(Default)]
struct MockSlowmodeUc {
    activated: Mutex<Vec<(String, serde_json::Value, i64, i32)>>,
}

#[async_trait]
impl ManageSlowmodeUseCase for MockSlowmodeUc {
    async fn activate(
        &self,
        guild_id: &str,
        previous_states: serde_json::Value,
        duration_secs: i64,
        imposed_rate: i32,
    ) -> Result<(), DomainError> {
        self.activated.lock().unwrap().push((
            guild_id.into(),
            previous_states,
            duration_secs,
            imposed_rate,
        ));
        Ok(())
    }
    async fn deactivate(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

#[derive(Default)]
struct MockLockdownUc {
    activated: Mutex<Vec<(String, serde_json::Value, i64)>>,
}

#[async_trait]
impl ManageLockdownUseCase for MockLockdownUc {
    async fn activate(
        &self,
        guild_id: &str,
        saved_states: serde_json::Value,
        duration_secs: i64,
    ) -> Result<(), DomainError> {
        self.activated
            .lock()
            .unwrap()
            .push((guild_id.into(), saved_states, duration_secs));
        Ok(())
    }
    async fn deactivate(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

fn grpc_with(
    q: Arc<MockQuarantineUc>,
    s: Arc<MockSlowmodeUc>,
    l: Arc<MockLockdownUc>,
) -> SecurityStateGrpc {
    SecurityStateGrpc {
        quarantine_uc: q,
        slowmode_uc: s,
        lockdown_uc: l,
    }
}

#[tokio::test]
async fn mark_quarantine_forwards_fields() {
    let q = Arc::new(MockQuarantineUc::default());
    let grpc = grpc_with(q.clone(), Arc::default(), Arc::default());
    grpc.mark_quarantine(Request::new(proto::MarkQuarantineRequest {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        timeout_secs: 600,
    }))
    .await
    .unwrap();
    assert_eq!(
        q.marked.lock().unwrap().as_slice(),
        &[("g1".into(), "u1".into(), 600)]
    );
}

#[tokio::test]
async fn mark_quarantine_error_maps_to_internal() {
    let q = Arc::new(MockQuarantineUc {
        fail: true,
        ..Default::default()
    });
    let grpc = grpc_with(q, Arc::default(), Arc::default());
    let err = grpc
        .mark_quarantine(Request::new(proto::MarkQuarantineRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            timeout_secs: 1,
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Internal);
}

#[tokio::test]
async fn lift_quarantine_forwards_fields() {
    let q = Arc::new(MockQuarantineUc::default());
    let grpc = grpc_with(q.clone(), Arc::default(), Arc::default());
    grpc.lift_quarantine(Request::new(proto::LiftQuarantineRequest {
        guild_id: "g1".into(),
        user_id: "u1".into(),
    }))
    .await
    .unwrap();
    assert_eq!(
        q.lifted.lock().unwrap().as_slice(),
        &[("g1".into(), "u1".into())]
    );
}

#[tokio::test]
async fn list_active_quarantines_maps_entries() {
    let q = Arc::new(MockQuarantineUc::default());
    q.active.lock().unwrap().push(ActiveQuarantine {
        guild_id: "g1".into(),
        user_id: "u1".into(),
    });
    let grpc = grpc_with(q, Arc::default(), Arc::default());
    let list = grpc
        .list_active_quarantines(Request::new(proto::ListActiveQuarantinesRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(list.entries.len(), 1);
    assert_eq!(list.entries[0].guild_id, "g1");
    assert_eq!(list.entries[0].user_id, "u1");
}

#[tokio::test]
async fn mark_slowmode_parses_json_states() {
    let s = Arc::new(MockSlowmodeUc::default());
    let grpc = grpc_with(Arc::default(), s.clone(), Arc::default());
    grpc.mark_slowmode(Request::new(proto::MarkSlowmodeRequest {
        guild_id: "g1".into(),
        previous_states_json: r#"[{"channel_id":"c1","rate":5}]"#.into(),
        duration_secs: 300,
        imposed_rate: 10,
    }))
    .await
    .unwrap();
    let calls = s.activated.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "g1");
    assert!(calls[0].1.is_array());
    assert_eq!(calls[0].2, 300);
    assert_eq!(calls[0].3, 10);
}

#[tokio::test]
async fn mark_slowmode_empty_json_becomes_null() {
    let s = Arc::new(MockSlowmodeUc::default());
    let grpc = grpc_with(Arc::default(), s.clone(), Arc::default());
    grpc.mark_slowmode(Request::new(proto::MarkSlowmodeRequest {
        guild_id: "g1".into(),
        previous_states_json: String::new(),
        duration_secs: 1,
        imposed_rate: 0,
    }))
    .await
    .unwrap();
    assert!(s.activated.lock().unwrap()[0].1.is_null());
}

#[tokio::test]
async fn mark_slowmode_invalid_json_is_invalid_argument() {
    let grpc = grpc_with(Arc::default(), Arc::default(), Arc::default());
    let err = grpc
        .mark_slowmode(Request::new(proto::MarkSlowmodeRequest {
            guild_id: "g1".into(),
            previous_states_json: "{not json".into(),
            duration_secs: 1,
            imposed_rate: 0,
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn mark_lockdown_parses_json_states() {
    let l = Arc::new(MockLockdownUc::default());
    let grpc = grpc_with(Arc::default(), Arc::default(), l.clone());
    grpc.mark_lockdown(Request::new(proto::MarkLockdownRequest {
        guild_id: "g1".into(),
        saved_states_json: r#"[{"channel_id":"c1"}]"#.into(),
        duration_secs: 120,
    }))
    .await
    .unwrap();
    let calls = l.activated.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "g1");
    assert!(calls[0].1.is_array());
    assert_eq!(calls[0].2, 120);
}
