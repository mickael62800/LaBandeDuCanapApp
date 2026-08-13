//! Gate d'acces unique du back-office : **superadmin uniquement**.
//!
//! Remplace l'ancienne pile RBAC multi-roles (`rbac` + `whitelist` +
//! `guild_auth` + `global_rbac`) par une regle unique :
//!
//!   1. Dev mode (`API_KEY` vide)                    → pass-through.
//!   2. `AuthKind::Internal` (bot/workers, Bearer)   → pass-through.
//!   3. Utilisateur web : son identite Discord doit figurer dans
//!      `SUPERADMIN_USER_IDS` (.env)                 → sinon **403**.
//!
//! Il n'y a plus de roles applicatifs, plus de table `api_user_guilds`, plus
//! d'invitations, plus de gating par guild : le back-office a exactement un
//! utilisateur humain autorise (ou plusieurs si l'env en liste plusieurs).
//!
//! # Fail-closed
//!
//! Si `SUPERADMIN_USER_IDS` est vide, AUCUN utilisateur web ne passe. C'est
//! volontaire : mieux vaut un back-office inaccessible qu'un back-office
//! ouvert. Les services internes continuent de fonctionner via l'`API_KEY`.
//!
//! # Identite
//!
//! Le `discord_user_id` resolu est injecte en extension `WebUser`. Les
//! handlers qui attribuent une action a son auteur (audit, `deleted_by`,
//! `granted_by`...) le lisent via `Option<Extension<WebUser>>` — `None`
//! signifiant « appel interne bot/worker ».

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use platform_common_api::auth_client::AccessOutcome;

use crate::sentinel::adapters::inbound::http::middleware::auth::AuthKind;
use crate::sentinel::bootstrap::state::SharedState;

const DISCORD_TOKEN_HEADER: &str = "x-discord-token";

/// Identite Discord du caller web, injectee en extension de requete.
/// Absente pour les appels internes (bot/workers) et en dev mode.
#[derive(Debug, Clone)]
pub struct WebUser {
    pub discord_user_id: String,
}

pub async fn superadmin_middleware(
    State(state): State<SharedState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Dev mode : pas d'API_KEY configuree, on ne casse pas le local.
    if state.api_key.is_empty() {
        return Ok(next.run(request).await);
    }

    // 2. Service interne de confiance (bot/workers via Bearer API_KEY).
    if request.extensions().get::<AuthKind>() == Some(&AuthKind::Internal) {
        return Ok(next.run(request).await);
    }

    // 3. Utilisateur web : on exige un token Discord exploitable.
    let (mut parts, body) = request.into_parts();
    let discord_token = match parts
        .headers
        .get(DISCORD_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
    {
        Some(t) if !t.is_empty() => t.to_string(),
        // Ni Bearer interne ni token web : `auth_middleware` a deja filtre ce
        // cas, on reste fail-closed par defense en profondeur.
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    // La decision appartient a `auth-api`. Ce processus ne resout plus
    // l'identite lui-meme et ne consulte plus SUPERADMIN_USER_IDS : deux
    // implementations de la meme regle finiraient par diverger, et c'est
    // exactement ce qui rendait Sentinel indispensable aux autres plateformes.
    let user_id = match state.auth.resolve(&discord_token).await {
        AccessOutcome::Granted(id) => id,
        AccessOutcome::Denied => {
            tracing::warn!(
                path = %parts.uri.path(),
                "superadmin: acces refuse par l'identite"
            );
            return Err(StatusCode::FORBIDDEN);
        }
        AccessOutcome::Unauthenticated => return Err(StatusCode::UNAUTHORIZED),
        // Identite injoignable : 503, PAS 403. Un refus ferait croire a
        // l'administrateur qu'il a perdu ses droits alors que c'est une panne.
        AccessOutcome::Unavailable => {
            tracing::warn!("superadmin: identite indisponible");
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    parts.extensions.insert(WebUser {
        discord_user_id: user_id,
    });

    Ok(next.run(Request::from_parts(parts, body)).await)
}

// La resolution d'identite (cache Redis + `GET /users/@me`) et la derivation
// SHA-256 de la cle de cache ont ete DEPLACEES dans `auth-core` /
// `auth-api/src/adapters/redis_stores.rs`, avec leurs tests. Elles ne sont pas
// dupliquees ici : deux implementations de la meme regle finissent toujours par
// diverger, et c'est justement ce que l'extraction de l'identite supprime.
