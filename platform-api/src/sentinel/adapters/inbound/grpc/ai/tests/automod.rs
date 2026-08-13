use super::*;

#[test]
fn proto_to_flags_round_trip_all_true() {
    let p = proto::DetectionFlags {
        spam: true,
        insult: true,
        profanity: false,
        link: true,
        phishing: true,
    };
    let f = proto_to_flags(p);
    assert!(f.spam && f.insult && f.link && f.phishing);
}

#[test]
fn proto_to_flags_round_trip_mixed() {
    let p = proto::DetectionFlags {
        spam: true,
        insult: false,
        profanity: false,
        link: true,
        phishing: false,
    };
    let f = proto_to_flags(p);
    assert!(f.spam);
    assert!(!f.insult);
    assert!(f.link);
    assert!(!f.phishing);
}

#[test]
fn action_to_proto_all_variants() {
    assert_eq!(action_to_proto(Action::None), proto::Action::None as i32);
    assert_eq!(action_to_proto(Action::Warn), proto::Action::Warn as i32);
    assert_eq!(
        action_to_proto(Action::Delete),
        proto::Action::Delete as i32
    );
    assert_eq!(action_to_proto(Action::Mute), proto::Action::Mute as i32);
    assert_eq!(action_to_proto(Action::Ban), proto::Action::Ban as i32);
}

#[test]
fn analysis_to_proto_full_mapping() {
    let a = MessageAnalysis {
        action: Action::Warn,
        reason: "spam".into(),
        score: 0.65,
        duration: Some(300),
        route:
            platform_core::sentinel::domain::services::moderation::automod_routing::Routing::Card,
        auto_action: false,
        severe: false,
        auto_delete_link: false,
    };
    let p = analysis_to_proto(a);
    assert_eq!(p.action, proto::Action::Warn as i32);
    assert_eq!(p.reason, "spam");
    assert!((p.score - 0.65).abs() < 1e-6);
    assert_eq!(p.duration, Some(300));
}

#[test]
fn analysis_to_proto_no_action() {
    let a = MessageAnalysis {
        action: Action::None,
        reason: String::new(),
        score: 0.0,
        duration: None,
        route:
            platform_core::sentinel::domain::services::moderation::automod_routing::Routing::None,
        auto_action: false,
        severe: false,
        auto_delete_link: false,
    };
    let p = analysis_to_proto(a);
    assert_eq!(p.action, proto::Action::None as i32);
    assert!(p.duration.is_none());
}

// ── RPC handler tests avec mock AnalyzeMessageUseCase ──

use async_trait::async_trait;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageCommand;
use platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Default)]
struct MockAnalyzeUc {
    calls: Mutex<Vec<AnalyzeMessageCommand>>,
}

#[async_trait]
impl AnalyzeMessageUseCase for MockAnalyzeUc {
    async fn analyze(&self, cmd: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError> {
        self.calls.lock().unwrap().push(cmd);
        Ok(MessageAnalysis {
            action: Action::Warn,
            reason: "spam".into(),
            score: 0.75,
            duration: None,
            route: platform_core::sentinel::domain::services::moderation::automod_routing::Routing::Card,
            auto_action: false,
            severe: false,
            auto_delete_link: false,
        })
    }
    async fn evaluate_flood(
        &self,
        _guild_id: &str,
        _flood_count: i32,
    ) -> Result<
        platform_core::sentinel::ports::inbound::ai::analyze_message::FloodDecision,
        DomainError,
    > {
        unimplemented!()
    }
    async fn evaluate_attachments(
        &self,
        _: &str,
        _: Vec<String>,
    ) -> Result<
        platform_core::sentinel::ports::inbound::ai::analyze_message::AttachmentDecision,
        DomainError,
    > {
        unimplemented!()
    }
    async fn evaluate_caps(
        &self,
        _: &str,
    ) -> Result<
        platform_core::sentinel::ports::inbound::ai::analyze_message::CapsDecision,
        DomainError,
    > {
        unimplemented!()
    }
}

fn make_req(guild_id: &str, user_id: &str, content: &str) -> Request<proto::AnalyzeMessageRequest> {
    Request::new(proto::AnalyzeMessageRequest {
        guild_id: guild_id.into(),
        channel_id: "c1".into(),
        user_id: user_id.into(),
        username: "alice".into(),
        content: content.into(),
        flags: None,
        message_id: "m1".into(),
        timestamp: "2026-01-01T00:00:00Z".into(),
        context_messages: vec![],
    })
}

#[tokio::test]
async fn analyze_message_rejects_empty_guild_id() {
    let g = AutomodGrpc {
        uc: Arc::new(MockAnalyzeUc::default()),
        broadcaster: Arc::new(EventBroadcaster::new()),
        adaptive_slowmode_repo: Arc::new(MockSlowmodeRepo::default()),
    };
    let err = g
        .analyze_message(make_req("", "u", "hello"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("guild_id"));
}

#[tokio::test]
async fn analyze_message_rejects_too_long_guild_id() {
    let g = AutomodGrpc {
        uc: Arc::new(MockAnalyzeUc::default()),
        broadcaster: Arc::new(EventBroadcaster::new()),
        adaptive_slowmode_repo: Arc::new(MockSlowmodeRepo::default()),
    };
    let long = "1".repeat(21);
    let err = g
        .analyze_message(make_req(&long, "u", "hello"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn analyze_message_rejects_empty_user_id() {
    let g = AutomodGrpc {
        uc: Arc::new(MockAnalyzeUc::default()),
        broadcaster: Arc::new(EventBroadcaster::new()),
        adaptive_slowmode_repo: Arc::new(MockSlowmodeRepo::default()),
    };
    let err = g
        .analyze_message(make_req("g", "", "hello"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("user_id"));
}

#[tokio::test]
async fn analyze_message_rejects_empty_content() {
    let g = AutomodGrpc {
        uc: Arc::new(MockAnalyzeUc::default()),
        broadcaster: Arc::new(EventBroadcaster::new()),
        adaptive_slowmode_repo: Arc::new(MockSlowmodeRepo::default()),
    };
    let err = g.analyze_message(make_req("g", "u", "")).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("content"));
}

#[tokio::test]
async fn analyze_message_delegates_to_uc_and_returns_analysis() {
    let uc = Arc::new(MockAnalyzeUc::default());
    let g = AutomodGrpc {
        uc: uc.clone(),
        broadcaster: Arc::new(EventBroadcaster::new()),
        adaptive_slowmode_repo: Arc::new(MockSlowmodeRepo::default()),
    };
    let resp = g
        .analyze_message(make_req("g1", "u1", "message content"))
        .await
        .unwrap();
    let inner = resp.into_inner();
    assert_eq!(inner.action, proto::Action::Warn as i32);
    assert_eq!(inner.reason, "spam");

    let calls = uc.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].guild_id, "g1".into());
    assert_eq!(calls[0].content, "message content");
}

#[tokio::test]
async fn analyze_message_maps_flags_from_proto() {
    let uc = Arc::new(MockAnalyzeUc::default());
    let g = AutomodGrpc {
        uc: uc.clone(),
        broadcaster: Arc::new(EventBroadcaster::new()),
        adaptive_slowmode_repo: Arc::new(MockSlowmodeRepo::default()),
    };
    let req = Request::new(proto::AnalyzeMessageRequest {
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "a".into(),
        content: "text".into(),
        flags: Some(proto::DetectionFlags {
            spam: true,
            insult: false,
            profanity: false,
            link: true,
            phishing: false,
        }),
        message_id: "m".into(),
        timestamp: "".into(),
        context_messages: vec![],
    });
    let _ = g.analyze_message(req).await.unwrap();
    let calls = uc.calls.lock().unwrap();
    assert!(calls[0].flags.spam);
    assert!(!calls[0].flags.insult);
    assert!(calls[0].flags.link);
    assert!(!calls[0].flags.phishing);
}

#[tokio::test]
async fn analyze_message_maps_context_messages() {
    let uc = Arc::new(MockAnalyzeUc::default());
    let g = AutomodGrpc {
        uc: uc.clone(),
        broadcaster: Arc::new(EventBroadcaster::new()),
        adaptive_slowmode_repo: Arc::new(MockSlowmodeRepo::default()),
    };
    let req = Request::new(proto::AnalyzeMessageRequest {
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "a".into(),
        content: "text".into(),
        flags: None,
        message_id: "m".into(),
        timestamp: "".into(),
        context_messages: vec![
            proto::ContextMessage {
                username: "prev1".into(),
                content: "a".into(),
            },
            proto::ContextMessage {
                username: "prev2".into(),
                content: "b".into(),
            },
        ],
    });
    let _ = g.analyze_message(req).await.unwrap();
    let calls = uc.calls.lock().unwrap();
    assert_eq!(calls[0].context_messages.len(), 2);
    assert_eq!(calls[0].context_messages[0].username, "prev1");
}

#[tokio::test]
async fn analyze_message_flags_none_defaults_all_false() {
    let uc = Arc::new(MockAnalyzeUc::default());
    let g = AutomodGrpc {
        uc: uc.clone(),
        broadcaster: Arc::new(EventBroadcaster::new()),
        adaptive_slowmode_repo: Arc::new(MockSlowmodeRepo::default()),
    };
    let _ = g.analyze_message(make_req("g", "u", "hi")).await.unwrap();
    let calls = uc.calls.lock().unwrap();
    assert!(!calls[0].flags.spam);
    assert!(!calls[0].flags.insult);
    assert!(!calls[0].flags.link);
    assert!(!calls[0].flags.phishing);
}

// ── Slowmode adaptatif ──

/// Repo en memoire : enregistre les appels pour verifier que le handler
/// delegue bien, et sur quelle cle.
#[derive(Default)]
struct MockSlowmodeRepo {
    marques: std::sync::Mutex<Vec<(String, String)>>,
    retires: std::sync::Mutex<Vec<String>>,
    contenu: std::sync::Mutex<Vec<(String, String)>>,
}

#[async_trait::async_trait]
impl platform_core::sentinel::ports::outbound::moderation::adaptive_slowmode_repository::AdaptiveSlowmodeRepository
    for MockSlowmodeRepo
{
    async fn mark(
        &self,
        guild_id: &str,
        channel_id: &str,
    ) -> Result<(), platform_core::sentinel::domain::errors::DomainError> {
        self.marques
            .lock()
            .unwrap()
            .push((guild_id.to_string(), channel_id.to_string()));
        Ok(())
    }
    async fn unmark(
        &self,
        channel_id: &str,
    ) -> Result<(), platform_core::sentinel::domain::errors::DomainError> {
        self.retires.lock().unwrap().push(channel_id.to_string());
        Ok(())
    }
    async fn list_all(
        &self,
    ) -> Result<Vec<(String, String)>, platform_core::sentinel::domain::errors::DomainError> {
        Ok(self.contenu.lock().unwrap().clone())
    }
}

fn grpc_avec(repo: Arc<MockSlowmodeRepo>) -> AutomodGrpc {
    AutomodGrpc {
        uc: Arc::new(MockAnalyzeUc::default()),
        broadcaster: Arc::new(EventBroadcaster::new()),
        adaptive_slowmode_repo: repo,
    }
}

#[tokio::test]
async fn mark_adaptive_slowmode_delegue_au_repo() {
    let repo = Arc::new(MockSlowmodeRepo::default());
    let g = grpc_avec(repo.clone());
    g.mark_adaptive_slowmode(tonic::Request::new(proto::AdaptiveSlowmodeChannel {
        guild_id: "g1".into(),
        channel_id: "c1".into(),
    }))
    .await
    .unwrap();
    assert_eq!(
        repo.marques.lock().unwrap().as_slice(),
        [("g1".to_string(), "c1".to_string())]
    );
}

#[tokio::test]
async fn unmark_adaptive_slowmode_ne_cle_que_sur_le_salon() {
    let repo = Arc::new(MockSlowmodeRepo::default());
    let g = grpc_avec(repo.clone());
    // guild_id vide : le retrait est cle par salon, la contrainte d'unicite
    // porte sur channel_id. Il ne doit pas etre rejete.
    g.unmark_adaptive_slowmode(tonic::Request::new(proto::AdaptiveSlowmodeChannel {
        guild_id: String::new(),
        channel_id: "c1".into(),
    }))
    .await
    .unwrap();
    assert_eq!(repo.retires.lock().unwrap().as_slice(), ["c1".to_string()]);
}

#[tokio::test]
async fn adaptive_slowmode_refuse_un_salon_vide() {
    let g = grpc_avec(Arc::new(MockSlowmodeRepo::default()));
    for req in [
        proto::AdaptiveSlowmodeChannel {
            guild_id: "g1".into(),
            channel_id: String::new(),
        },
        proto::AdaptiveSlowmodeChannel {
            guild_id: String::new(),
            channel_id: String::new(),
        },
    ] {
        let err = g
            .mark_adaptive_slowmode(tonic::Request::new(req))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }
}

#[tokio::test]
async fn list_adaptive_slowmode_rend_les_paires() {
    let repo = Arc::new(MockSlowmodeRepo::default());
    repo.contenu
        .lock()
        .unwrap()
        .push(("g1".into(), "c1".into()));
    let g = grpc_avec(repo.clone());
    let resp = g
        .list_adaptive_slowmode(tonic::Request::new(proto::ListAdaptiveSlowmodeRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp.channels.len(), 1);
    assert_eq!(resp.channels[0].guild_id, "g1");
    assert_eq!(resp.channels[0].channel_id, "c1");
}
