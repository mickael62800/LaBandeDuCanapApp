//! Presence en direct, surface publique.
//!
//! # Ce qui est publie et pourquoi
//!
//! Le bot publie tous les salons en marquant ceux que @everyone ne peut pas
//! voir (`restreint`) — lui seul connait les permissions Discord. Deux routes
//! s'en servent differemment :
//!
//!   - `public_presence` : anonyme, ecarte systematiquement les restreints ;
//!   - `member_presence` : authentifiee, les inclut en les signalant.
//!
//! Deux routes plutot qu'une seule au contenu variable selon l'en-tete
//! d'authentification : une route dont la reponse depend d'un jeton optionnel
//! fuit au premier oubli, et l'oubli ne se voit pas. Ici la route anonyme ne
//! sait meme pas construire la reponse etendue.
//!
//! Le DTO expose les pseudos mais PAS les identifiants Discord, comme les
//! autres surfaces publiques : un pseudo suffit a afficher une pastille,
//! l'identifiant permettrait de retrouver la personne hors du serveur.
//!
//! Une section vide est le cas normal (personne en vocal, bot redemarre,
//! Redis indisponible). Elle ne remonte jamais d'erreur : la page membre doit
//! s'afficher entiere meme quand cette brique est muette.

use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::Serialize;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::handlers::community::public_guard::ensure_guild_id;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::CommunityState;
use platform_core::sentinel::domain::errors::DomainError;

/// Salons ecrits remontes. Au-dela, la liste cesse d'informer.
const TEXT_CHANNELS: i64 = 5;

#[derive(Debug, Serialize)]
pub struct VoiceMemberDto {
    pub username: String,
    /// Micro coupe, quelle qu'en soit la cause. La page n'a pas besoin de
    /// distinguer une coupure volontaire d'une sanction — et l'afficher
    /// exposerait une decision de moderation.
    pub muted: bool,
    pub streaming: bool,
    pub video: bool,
}

#[derive(Debug, Serialize)]
pub struct VoiceChannelDto {
    pub channel_name: String,
    pub members: Vec<VoiceMemberDto>,
    /// Salon reserve sur Discord. Toujours `false` sur la route anonyme.
    pub restricted: bool,
}

#[derive(Debug, Serialize)]
pub struct TextChannelDto {
    pub channel_name: String,
    pub recent_authors: Vec<String>,
    pub last_message_at: String,
}

#[derive(Debug, Serialize)]
pub struct PresenceDto {
    pub voice: Vec<VoiceChannelDto>,
    pub voice_total: usize,
    pub text: Vec<TextChannelDto>,
}

/// GET /api/public/presence/{guild_id} — visiteurs anonymes.
pub async fn public_presence(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
) -> Result<Json<PresenceDto>, ApiError> {
    presence_dto(state, guild_id, false).await
}

/// GET /api/presence/{guild_id} — membres connectes.
///
/// Inclut les salons reserves : un membre du serveur y a de toute facon acces
/// sur Discord, les lui masquer ici ne protegeait rien et donnait une image
/// fausse de qui est connecte.
pub async fn member_presence(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(guild_id): Path<String>,
) -> Result<Json<PresenceDto>, ApiError> {
    // Le contexte user est exige : sans lui, cette route serait la route
    // publique avec les salons prives en plus.
    if user.is_none() {
        return Err(ApiError(DomainError::Forbidden(
            "connexion Discord requise".into(),
        )));
    }
    presence_dto(state, guild_id, true).await
}

async fn presence_dto(
    state: CommunityState,
    guild_id: String,
    inclure_restreints: bool,
) -> Result<Json<PresenceDto>, ApiError> {
    ensure_guild_id(&guild_id)?;

    let presence = state.presence_uc.voice(&guild_id).await?.map(|p| {
        if inclure_restreints {
            p
        } else {
            p.sans_restreints()
        }
    });
    let text = state
        .presence_uc
        .text_activity(&guild_id, TEXT_CHANNELS)
        .await?;

    let (voice, voice_total) = match presence {
        Some(p) => {
            let total = p.total_members();
            let salons = p
                .occupied_channels()
                .into_iter()
                .map(|c| VoiceChannelDto {
                    channel_name: c.channel_name.clone(),
                    restricted: c.restreint,
                    members: c
                        .members
                        .iter()
                        .map(|m| VoiceMemberDto {
                            username: m.username.clone(),
                            muted: !m.can_speak(),
                            streaming: m.streaming,
                            video: m.video,
                        })
                        .collect(),
                })
                .collect();
            (salons, total)
        }
        // Instantane absent ou perime : on renvoie du vide plutot qu'une
        // erreur, la page masquera simplement la section.
        None => (vec![], 0),
    };

    Ok(Json(PresenceDto {
        voice,
        voice_total,
        text: text
            .into_iter()
            .map(|t| TextChannelDto {
                channel_name: t.channel_name,
                recent_authors: t.recent_authors,
                last_message_at: t.last_message_at.to_rfc3339(),
            })
            .collect(),
    }))
}
