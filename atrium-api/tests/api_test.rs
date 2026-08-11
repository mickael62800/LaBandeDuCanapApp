use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use atrium_api::{grpc::WelcomeGrpc, router_with_state, AppConfig, AppState};
use atrium_core::{
    domain::{WelcomeError, WelcomeReply, WelcomeRequest},
    ports::inbound::GenerateWelcomeReplyUseCase,
};
use atrium_proto::welcome::v1::{
    welcome_service_server::WelcomeService, ConversationScope as ProtoScope, GenerateReplyRequest,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tonic::Request as TonicRequest;
use tower::ServiceExt;

/// Mock du UseCase `GenerateWelcomeReplyUseCase` pour simuler le comportement du métier.
struct MockWelcomeUseCase {
    should_fail: bool,
}

impl MockWelcomeUseCase {
    fn new(should_fail: bool) -> Self {
        Self { should_fail }
    }
}

#[async_trait]
impl GenerateWelcomeReplyUseCase for MockWelcomeUseCase {
    async fn reply(&self, req: WelcomeRequest) -> Result<WelcomeReply, WelcomeError> {
        if self.should_fail {
            return Err(WelcomeError::Missing("guild_id"));
        }
        Ok(WelcomeReply {
            content: format!(
                "Bienvenue {} sur le serveur {} !",
                req.member_display_name, req.guild_id
            ),
            generated_by_ai: true,
        })
    }
}

fn setup_test_app(should_fail: bool) -> axum::Router {
    let config = AppConfig::dummy();
    let mock_use_case = Arc::new(MockWelcomeUseCase::new(should_fail));
    let state = Arc::new(AppState {
        // Le service d'apaisement n'est pas exerce par ces tests, mais il est
        // obligatoire dans l'etat : on prend le vrai, qui ne joint le modele
        // qu'a l'appel. Aucun de ces tests ne l'appelle.
        calming: atrium_api::calming_use_case(&config),
        summary: atrium_api::summary_use_case(&config),
        config,
        welcome: mock_use_case,
        rag: None,
        budget: None,
        control: None,
        memory: None,
        // Aucune base dans ces tests : les routes d'administration repondent
        // 503, ce qui est exactement le comportement attendu sans dependance.
        config_pool: None,
    });
    router_with_state(state)
}

fn create_request(method: &str, uri: &str, body: Body) -> Request<Body> {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .unwrap();

    let dummy_addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(dummy_addr));
    req
}
#[tokio::test]
async fn test_health_endpoint() {
    let app = setup_test_app(false);

    let req = create_request("GET", "/health", Body::empty());
    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn test_grpc_generate_reply_success() {
    let mock_use_case = Arc::new(MockWelcomeUseCase::new(false));
    let grpc_service = WelcomeGrpc::new(mock_use_case);

    let req = TonicRequest::new(GenerateReplyRequest {
        guild_id: "12345".into(),
        member_id: "u1".into(),
        member_display_name: "Bob".into(),
        channel_id: "c1".into(),
        scope: ProtoScope::General.into(),
        member_message: "Salut".into(),
        server_context: "Bienvenue sur le serveur".into(),
    });

    let res = grpc_service.generate_reply(req).await.unwrap();
    let inner = res.into_inner();

    assert_eq!(inner.reply, "Bienvenue Bob sur le serveur 12345 !");
    assert!(inner.generated_by_ai);
}

#[tokio::test]
async fn test_grpc_generate_reply_error() {
    let mock_use_case = Arc::new(MockWelcomeUseCase::new(true));
    let grpc_service = WelcomeGrpc::new(mock_use_case);

    let req = TonicRequest::new(GenerateReplyRequest {
        guild_id: "12345".into(),
        member_id: "u1".into(),
        member_display_name: "Bob".into(),
        channel_id: "c1".into(),
        scope: ProtoScope::Direct.into(),
        member_message: "Salut".into(),
        server_context: "".into(),
    });

    let res = grpc_service.generate_reply(req).await;
    assert!(res.is_err());
    let status = res.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}
