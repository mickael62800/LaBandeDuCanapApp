//! Sondes d'autorisation sans corps : le statut HTTP porte toute la reponse.
//!
//! Depuis le passage du back-office en superadmin-only, il n'y a plus de role
//! applicatif ni de gate par composant a evaluer : `superadmin_middleware` a
//! deja refuse (403) tout appelant web absent de `SUPERADMIN_USER_IDS`.
//! Atteindre ces handlers suffit donc a etre autorise.

use axum::http::StatusCode;

/// GET /api/auth/nexus-access
///
/// Cible de la directive `auth_request` de nginx pour la passerelle
/// `/nexus-api/`. Pourquoi ici et pas dans nexus-api : nexus-api n'a aucune
/// notion d'utilisateur (une seule cle statique). Sentinel reste la source de
/// verite unique de l'identite ; nginx lui demande son avis avant de relayer,
/// puis injecte lui-meme la cle Nexus cote serveur. Le navigateur ne la voit
/// jamais.
///
/// L'en-tete `X-Guild-Id` n'a plus d'influence sur la decision.
pub async fn nexus_access() -> StatusCode {
    StatusCode::OK
}

/// GET /api/auth/check-access
///
/// Sondage du front juste apres l'OAuth : 200 = le compte Discord connecte est
/// superadmin et peut entrer dans le back-office, 403 = il ne l'est pas (le
/// middleware repond avant d'atteindre ce handler).
pub async fn check_access() -> StatusCode {
    StatusCode::OK
}
