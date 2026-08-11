//! Audit mutualise des actions Docker : identification de l'operateur et
//! journalisation du RESULTAT reel de chaque action.

use axum::http::HeaderMap;

use crate::{ApiError, AppState};

/// Identifiant Discord de l'operateur, remonte par nginx depuis auth_request.
///
/// ops-api ne sait pas resoudre une session Discord. Sans cette remontee,
/// l'audit des actions destructives perdrait son auteur : on saurait qu'un
/// conteneur a ete supprime, jamais par qui.
pub(crate) fn actor_from(headers: &HeaderMap) -> String {
    headers
        .get("x-actor-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .unwrap_or("inconnu")
        .to_owned()
}

/// Journalise le RESULTAT d'une action Docker.
///
/// Ecrit dans `server_events` (page Securite serveur) APRES l'appel, avec un
/// champ `success` : une operation refusee ou en erreur n'apparait plus comme
/// executee. Awaited (et non plus detachee via `tokio::spawn`) pour que la trace
/// durable existe avant qu'on reponde a l'operateur ; une erreur d'ecriture de
/// l'audit lui-meme n'est que journalisee — elle ne fait pas echouer une action
/// Docker qui, elle, a bien eu lieu.
pub(crate) async fn record_docker_audit(
    state: &AppState,
    actor: &str,
    action: &str,
    target: &str,
    success: bool,
    error: Option<&str>,
) {
    tracing::info!(
        target: "audit::docker",
        actor = %actor,
        action = action,
        target = target,
        success = success,
        "action d'administration Docker"
    );
    // Un echec est toujours visible (warn) ; les purges/suppressions reussies le
    // sont aussi (irreversibles) ; le reste en info.
    let severite = if !success || action.contains("prune") || action.contains("remove") {
        "warn"
    } else {
        "info"
    };
    let details = serde_json::json!({ "success": success, "error": error });
    if let Err(err) = state
        .server_events
        .record(
            actor,
            None,
            &format!("docker.{action}"),
            Some(target),
            severite,
            details,
        )
        .await
    {
        tracing::warn!(error = %err, "journalisation de l'action Docker impossible");
    }
}

/// Execute une action Docker et journalise son resultat (succes ou echec).
///
/// Mutualise le couple execution/audit : les handlers n'ont plus a se souvenir
/// d'auditer ni dans quel ordre, et l'audit reflete toujours l'issue reelle.
pub(crate) async fn audited<T>(
    state: &AppState,
    actor: &str,
    action: &str,
    target: &str,
    op: impl std::future::Future<Output = Result<T, ops_core::domain::errors::DomainError>>,
) -> Result<T, ApiError> {
    let result = op.await;
    let error = result.as_ref().err().map(|e| e.to_string());
    record_docker_audit(state, actor, action, target, result.is_ok(), error.as_deref()).await;
    result.map_err(ApiError::from)
}
