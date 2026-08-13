//! En-tetes de securite et CORS, communs aux deux APIs.
//!
//! Les deux ne servent que du JSON : aucun script ni ressource n'a de raison
//! d'etre charge depuis ces domaines, d'ou la CSP `default-src 'none'`.

use axum::http::header;
use axum::http::HeaderValue;
use axum::http::Method;
use axum::Router;
use tower_http::cors::AllowOrigin;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;

/// Construit la couche CORS a partir d'une liste d'origines.
///
/// `origins` accepte `*`, une liste separee par des virgules, ou une chaine
/// vide (repli sur `defauts`).
///
/// `*` est INCOMPATIBLE avec `allow_credentials(true)` : le combo autoriserait
/// n'importe quelle origine a envoyer les cookies de session ou l'en-tete
/// `Authorization`. Les credentials sont donc desactives des que la config est
/// en wildcard, avec un avertissement explicite.
pub fn build_cors(origins: &str, defauts: &[&str], extra_headers: &[&'static str]) -> CorsLayer {
    let wildcard = origins == "*";
    if wildcard {
        tracing::warn!(
            "CORS en mode permissif (*) SANS credentials. \
             Lister les origines exactes pour autoriser les cookies."
        );
    }

    let allow_origin = if wildcard {
        AllowOrigin::any()
    } else if origins.is_empty() {
        tracing::info!("Origines CORS non configurees — repli sur les valeurs par defaut");
        AllowOrigin::list(
            defauts
                .iter()
                .filter_map(|o| o.parse::<HeaderValue>().ok())
                .collect::<Vec<_>>(),
        )
    } else {
        AllowOrigin::list(
            origins
                .split(',')
                .filter_map(|o| o.trim().parse::<HeaderValue>().ok())
                .collect::<Vec<_>>(),
        )
    };

    let mut headers = vec![
        header::CONTENT_TYPE,
        header::AUTHORIZATION,
        header::HeaderName::from_static("x-request-id"),
    ];
    headers.extend(
        extra_headers
            .iter()
            .map(|h| header::HeaderName::from_static(h)),
    );

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(headers)
        .allow_credentials(!wildcard)
        .max_age(std::time::Duration::from_secs(3600))
}

/// Applique les en-tetes de securite communs a tout le routeur.
pub fn security_headers<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
            ),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // `CorsLayer` n'expose pas sa configuration : on verifie surtout que la
    // construction ne panique sur aucune des trois formes d'entree, y compris
    // avec des origines invalides melangees a des valides.
    #[test]
    fn build_cors_accepte_les_trois_formes() {
        let defauts = ["http://localhost:5173"];
        let _ = build_cors("*", &defauts, &[]);
        let _ = build_cors("", &defauts, &[]);
        let _ = build_cors("https://a.example, https://b.example", &defauts, &[]);
        // Origine invalide ignoree sans panique.
        let _ = build_cors("pas une origine\u{7f}, https://ok.example", &defauts, &[]);
    }

    #[test]
    fn build_cors_accepte_des_en_tetes_supplementaires() {
        let _ = build_cors("", &["http://localhost:3000"], &["x-discord-token"]);
    }
}
