use crate::sentinel::adapters::inbound::http::dto::moderation::infractions::InfractionQueryParams;
use crate::sentinel::adapters::inbound::http::dto::moderation::infractions::InfractionResponseDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::normalize_limit;
use crate::sentinel::adapters::inbound::http::helpers::normalize_offset;
use crate::sentinel::adapters::inbound::http::helpers::ok_response;
use crate::sentinel::bootstrap::state::ModerationState;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use platform_core::sentinel::ports::inbound::moderation::manage_infractions::InfractionFilters;

pub async fn list_infractions(
    State(state): State<ModerationState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<InfractionQueryParams>,
) -> Result<Json<Vec<InfractionResponseDto>>, ApiError> {
    // Validation

    let filters = InfractionFilters {
        user_id: params.user_id,
        action: params.action,
        limit: normalize_limit(params.limit, 50, 200),
        offset: normalize_offset(params.offset),
    };

    let infractions = state
        .infractions_uc
        .list_infractions(&guild_id, filters)
        .await?;
    Ok(map_to_dtos(infractions))
}

pub async fn delete_infraction(
    State(state): State<ModerationState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Recuperer l'infraction avant suppression pour le DM ET pour le gate RBAC
    let infraction = state.infractions_uc.find_by_id(&id).await?;

    // Phase 7 B — Gate RBAC : moderator+ requis. L'infraction porte son
    // propre guild_id, donc on fetch d'abord puis on verifie le role
    // via check_role_for_guild (pattern "ressource-id-based").

    let deleted = state.infractions_uc.delete_infraction(&id).await?;
    if !deleted {
        return Err(
            platform_core::sentinel::domain::errors::DomainError::NotFound(
                "Infraction introuvable".into(),
            )
            .into(),
        );
    }

    // Envoyer un DM a l'utilisateur pour l'informer de la grace
    if let Some(inf) = infraction {
        // Phase 1 sync : si c etait un proposal de ban, on previent le
        // bot pour qu il edite le message Discord (cf.
        // SYNC_DISCORD_WEB_DESIGN.md). L `action_id` = id de
        // l infraction supprimee.
        if inf.action.as_str() == "ban" {
            if let Ok(action_uuid) = uuid::Uuid::parse_str(&id) {
                state.broadcaster.broadcast(
                    "moderation.ban.cancelled",
                    serde_json::json!({
                        "action_id": action_uuid,
                        "guild_id": &inf.guild_id,
                        "target_id": &inf.user_id,
                        "actor": { "user_id": "desktop", "source": "web" },
                    }),
                );
            }
        }

        let message = format!(
            "Bonne nouvelle ! Votre avertissement a ete annule.\n\
            **Raison initiale** : {}\n\
            **Type** : {}\n\n\
            Cette infraction a ete retiree de votre dossier.",
            inf.reason,
            inf.action.as_str(),
        );
        if let Err(e) = state.discord_api.send_dm(&inf.user_id, &message).await {
            tracing::warn!("Echec envoi DM de grace a {}: {e}", inf.user_id);
        }
    }

    Ok(ok_response())
}
