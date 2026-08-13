//! Verrou mono-serveur.
//!
//! Cette installation ne sert qu'un seul serveur Discord. Le modele de
//! donnees conserve sa colonne `guild_id` — la retirer serait un refactor de
//! centaines de fichiers pour aucun gain, puisqu'elle vaudrait toujours la
//! meme chose — mais la surface HTTP, elle, n'accepte qu'une valeur.
//!
//! Un point de passage UNIQUE plutot qu'un controle recopie dans chaque
//! handler : c'est la seule facon d'etre sur que la centaine de routes
//! portant un `{guild_id}` soit couverte, y compris celles ajoutees demain.
//!
//! Le verrou couvre l'URL et les corps JSON. Pour ces derniers, il valide
//! recursivement chaque champ nomme `guild_id`, puis reconstruit le corps afin
//! que l'extracteur type du handler le deserialize normalement. Les corps non
//! JSON (multipart, binaires) ne sont pas lus.
//!
//! # Pourquoi ici et pas seulement dans le front
//!
//! Masquer le selecteur de serveur ne protege rien : l'API reste joignable
//! directement. Sans ce verrou, quelqu'un possedant un jeton valide pourrait
//! lire ou ecrire les donnees d'un autre serveur ou le bot serait installe.
//!
//! # Interaction avec `guild_auth`
//!
//! `guild_auth_middleware` verifie que l'APPELANT appartient a la guilde
//! demandee. Ce verrou-ci verifie que la GUILDE est celle de l'installation.
//! Les deux sont complementaires : le premier protege les membres les uns des
//! autres, le second cloisonne l'installation.

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use crate::sentinel::bootstrap::state::SharedState;

/// Refuse toute requete portant un `guild_id` autre que celui configure.
///
/// Laisse passer :
///   - les requetes sans `guild_id` (endpoints globaux, sante, OAuth) ;
///   - toutes les requetes si `guild_id` n'est pas configure, pour ne pas
///     bloquer une installation qui n'a pas encore renseigne la variable.
pub async fn single_guild_middleware(
    State(state): State<SharedState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let attendu = state.guild_id.clone();
    if attendu.is_empty() {
        return Ok(next.run(request).await);
    }

    let (mut parts, body) = request.into_parts();

    // Source autoritaire : le parametre de route matche par axum. L'heuristique
    // sur le chemin sert aux routes qui ne declarent pas `{guild_id}` mais
    // transportent quand meme un identifiant.
    let path = parts.uri.path().to_string();
    let trouve = guild_id_from_route_param(&mut parts, &state)
        .await
        .or_else(|| guild_id_from_path(&path));

    if let Some(gid) = trouve {
        if gid != attendu {
            tracing::warn!(
                guild_id = %gid,
                attendu = %attendu,
                path = %path,
                "mono-serveur : requete refusee pour une autre guilde"
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let body = if is_json_content_type(&parts.headers) {
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(gid) = foreign_guild_id(&json, &attendu) {
                tracing::warn!(
                    guild_id = %gid,
                    attendu = %attendu,
                    path = %path,
                    "mono-serveur : corps JSON refuse pour une autre guilde"
                );
                return Err(StatusCode::FORBIDDEN);
            }
        }
        Body::from(bytes)
    } else {
        body
    };

    Ok(next.run(Request::from_parts(parts, body)).await)
}

fn is_json_content_type(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .is_some_and(|mime| mime == "application/json" || mime.ends_with("+json"))
}

fn foreign_guild_id(value: &serde_json::Value, expected: &str) -> Option<String> {
    match value {
        serde_json::Value::Object(fields) => fields.iter().find_map(|(name, value)| {
            if name == "guild_id" {
                let supplied = match value {
                    serde_json::Value::String(value) => Some(value.clone()),
                    serde_json::Value::Number(value) => Some(value.to_string()),
                    _ => None,
                };
                supplied.filter(|value| value != expected)
            } else {
                foreign_guild_id(value, expected)
            }
        }),
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| foreign_guild_id(value, expected)),
        _ => None,
    }
}

/// Premier segment du chemin qui ressemble a un identifiant Discord.
///
/// Volontairement plus strict que l'heuristique de `guild_auth` : ici un
/// faux positif provoque un refus, donc on n'accepte qu'un snowflake
/// plausible (17 a 20 chiffres). Un identifiant plus court est ignore plutot
/// que de bloquer une route legitime.
/// Valeur du parametre de route nomme exactement `guild_id`, tel que matche
/// par le routeur axum. Source autoritaire (pas de devinette) : ne renvoie une
/// valeur que si la route declare reellement un `{guild_id}`.
async fn guild_id_from_route_param(
    parts: &mut axum::http::request::Parts,
    state: &SharedState,
) -> Option<String> {
    use axum::extract::{FromRequestParts, RawPathParams};
    let params = RawPathParams::from_request_parts(parts, state).await.ok()?;
    params
        .iter()
        .find(|(k, _)| *k == "guild_id")
        .map(|(_, v)| v.to_string())
}

/// Segments qui signalent que l'identifiant SUIVANT n'est PAS un guild_id mais
/// l'id d'une autre entite (salon, message, utilisateur...).
///
/// Sans ce garde, une route comme `/api/voice-channels/by-channel/{channel_id}/purge`
/// voyait son `channel_id` (un snowflake de 17-20 chiffres, indiscernable d'un
/// guild_id) pris pour le guild et refusait la requete a tort (403). La source
/// autoritaire reste le parametre de route `{guild_id}` ; cette heuristique
/// n'est qu'un filet, et ne doit pas se declencher sur un id d'entite.
const NON_GUILD_MARKERS: &[&str] = &[
    "by-channel",
    "by-id",
    "by-message",
    "by-message-id",
    "by-user",
    "detail",
    "replies",
    "reports",
    "bans",
    "co-admins",
    "invites",
];

fn guild_id_from_path(path: &str) -> Option<String> {
    let segs: Vec<&str> = path.split('/').collect();
    segs.iter().enumerate().find_map(|(i, seg)| {
        let ressemble = (17..=20).contains(&seg.len()) && seg.chars().all(|c| c.is_ascii_digit());
        if !ressemble {
            return None;
        }
        // Ignore l'id s'il suit un marqueur d'entite : c'est un channel_id,
        // message_id, user_id... pas un guild_id.
        let precedent = i.checked_sub(1).map(|j| segs[j]).unwrap_or("");
        if NON_GUILD_MARKERS.contains(&precedent) {
            return None;
        }
        Some(seg.to_string())
    })
}

#[cfg(test)]
#[path = "tests/single_guild.rs"]
mod tests;
