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
///
/// **Un jeton attendu vide ne valide rien.** Sans cette garde, `matches(h, "")`
/// renvoyait `true` des que le client envoyait `Authorization: Bearer ` — le
/// prefixe seul, suivi d'une chaine vide. Une API dont le jeton n'est pas
/// configure (variable definie mais vide, ce que `std::env::var` rend en
/// `Ok("")`) s'ouvrait donc a qui connaissait l'astuce, sans qu'aucun garde
/// n'ait l'air absent a la relecture.
///
/// La configuration doit refuser un secret vide en amont ; ceci est la seconde
/// barriere, dans le seul endroit que toutes les APIs traversent.
pub fn matches(headers: &HeaderMap, expected_token: &str) -> bool {
    use subtle::ConstantTimeEq;

    if expected_token.is_empty() {
        return false;
    }
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

    #[test]
    fn un_jeton_attendu_vide_ne_valide_jamais() {
        // `Bearer ` (prefixe seul) donnait un jeton vide, egal au jeton attendu
        // vide : l'API s'ouvrait entierement.
        let mut prefixe_seul = HeaderMap::new();
        prefixe_seul.insert(AUTHORIZATION, "Bearer ".parse().unwrap());
        assert!(!matches(&prefixe_seul, ""));

        assert!(!matches(&HeaderMap::new(), ""));

        let mut quelconque = HeaderMap::new();
        quelconque.insert(AUTHORIZATION, "Bearer nimporte".parse().unwrap());
        assert!(!matches(&quelconque, ""));
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
