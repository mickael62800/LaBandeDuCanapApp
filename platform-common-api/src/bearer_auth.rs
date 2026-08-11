//! Verification commune des jetons Bearer internes.

use axum::extract::{Request, State};
use axum::http::{header::AUTHORIZATION, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[derive(Clone)]
pub struct RequiredBearerToken(String);

impl RequiredBearerToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }
}

#[derive(Clone)]
pub struct OptionalBearerToken(Option<String>);

impl OptionalBearerToken {
    pub fn new(token: Option<String>) -> Self {
        Self(token)
    }
}

/// Verifie strictement `Authorization: Bearer <token>`.
pub fn matches(headers: &HeaderMap, expected_token: &str) -> bool {
    use subtle::ConstantTimeEq;

    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| token.as_bytes().ct_eq(expected_token.as_bytes()).into())
}

/// Middleware pour un groupe de routes protege par un Bearer obligatoire.
pub async fn require(
    State(expected): State<RequiredBearerToken>,
    request: Request,
    next: Next,
) -> Response {
    if matches(request.headers(), &expected.0) {
        return next.run(request).await;
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "jeton API invalide" })),
    )
        .into_response()
}

/// Variante pour les APIs qui autorisent explicitement un mode developpement
/// sans jeton configure.
pub async fn require_optional(
    State(expected): State<OptionalBearerToken>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    match expected.0.as_deref() {
        None => Ok(next.run(request).await),
        Some(token) if matches(request.headers(), token) => Ok(next.run(request).await),
        Some(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    #[test]
    fn accepte_uniquement_le_bon_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer secret".parse().unwrap());
        assert!(matches(&headers, "secret"));
        assert!(!matches(&headers, "autre"));
    }

    #[test]
    fn refuse_les_formes_invalides() {
        assert!(!matches(&HeaderMap::new(), "secret"));
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Basic secret".parse().unwrap());
        assert!(!matches(&headers, "secret"));
    }

    #[tokio::test]
    async fn protege_un_groupe_sans_bloquer_les_routes_publiques() {
        let protected = Router::new()
            .route("/private", get(|| async { StatusCode::OK }))
            .route_layer(axum::middleware::from_fn_with_state(
                RequiredBearerToken::new("secret"),
                require,
            ));
        let app = Router::new()
            .route("/health", get(|| async { StatusCode::OK }))
            .merge(protected);

        let public = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(public.status(), StatusCode::OK);

        let denied = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/private")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

        let allowed = app
            .oneshot(
                Request::builder()
                    .uri("/private")
                    .header(AUTHORIZATION, "Bearer secret")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(allowed.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn jeton_optionnel_autorise_uniquement_le_mode_sans_configuration() {
        let route = || Router::new().route("/", get(|| async { StatusCode::OK }));
        let open = route().route_layer(axum::middleware::from_fn_with_state(
            OptionalBearerToken::new(None),
            require_optional,
        ));
        assert_eq!(
            open.oneshot(Request::new(axum::body::Body::empty()))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );

        let protected = route().route_layer(axum::middleware::from_fn_with_state(
            OptionalBearerToken::new(Some("secret".into())),
            require_optional,
        ));
        assert_eq!(
            protected
                .oneshot(Request::new(axum::body::Body::empty()))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
}
