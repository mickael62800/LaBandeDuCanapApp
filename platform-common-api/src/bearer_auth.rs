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

// `OptionalBearerToken` / `require_optional` ont ete supprimes.
//
// C'etait la variante « mode developpement sans jeton configure » : jeton
// absent = toutes les routes passent. `nexus-api` en etait l'unique porteur,
// et l'a paye — cle vide au compose, `None` en memoire, et le cycle de vie des
// conteneurs de l'hote servi sans authentification. Il exige desormais sa cle
// au demarrage, ce qui ne laissait plus d'appelant a ce middleware.
//
// Ne pas le reintroduire : un socle qui offre un mode fail-open finit par etre
// utilise en production, et l'appelant qui s'en sert ne le dit nulle part.
// Une API qui veut demarrer sans secret doit l'assumer chez elle, visiblement.

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
}
