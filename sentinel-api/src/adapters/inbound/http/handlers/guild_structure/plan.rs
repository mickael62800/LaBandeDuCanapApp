//! Lecture de l'arborescence d'un serveur et application d'un plan de creation.

use axum::Json;
use std::collections::HashMap;

use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use sentinel_core::domain::entities::system::channel_access::{
    overwrites_for, AccessMode, ChannelAccess,
};
use sentinel_core::domain::entities::system::channel_plan::{
    ChannelPlan, PlannedChannel, PlannedChannelKind,
};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::discord_api::{DiscordRoleInfo, NewChannel};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::validation;
use crate::bootstrap::state::CommunityState;

// ── Lecture de l'existant ──

/// Un salon du serveur, tel qu'affiche par le constructeur web.
#[derive(Debug, Serialize)]
pub struct ExistingChannelDto {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub position: i64,
}

/// GET /api/guild-structure/{guild_id} — arborescence actuelle du serveur.
///
/// Sert de contexte au constructeur : on ne compose pas des salons a l'aveugle,
/// on les ajoute a cote de ce qui existe deja.
///
/// Gate Admin meme en lecture : la liste est obtenue avec le token du BOT,
/// donc elle contient aussi les salons prives que l'appelant ne verrait pas
/// sur Discord. Sans gate, n'importe quel compte du panel apprendrait
/// l'existence et le nom des salons de moderation ou de direction.
pub async fn get_structure(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<ExistingChannelDto>>, ApiError> {
    let channels = state.discord_api.list_all_channels(&guild_id).await?;
    Ok(Json(
        channels
            .into_iter()
            .map(|c| ExistingChannelDto {
                id: c.id,
                name: c.name,
                kind: c.kind,
                position: c.position,
            })
            .collect(),
    ))
}

/// GET /api/guild-structure/{guild_id}/roles — roles du serveur, lus EN DIRECT
/// aupres de Discord.
///
/// On ne sert pas ici la table `discord_roles` (cache synchronise) : composer
/// des permissions avec une liste en retard ferait poser des droits sur un role
/// disparu, ou omettrait celui qu'on vient de creer.
/// Gate Admin : la hierarchie complete des roles renseigne sur l'organisation
/// interne du serveur, et cet ecran est de toute facon reserve aux Admins.
pub async fn list_roles(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<DiscordRoleInfo>>, ApiError> {
    Ok(Json(state.discord_api.list_roles(&guild_id).await?))
}

// ── Application d'un plan ──

/// Un element du plan, tel qu'envoye par le web (le `kind` y est une chaine).
#[derive(Debug, Deserialize)]
pub struct PlanItemRequest {
    pub key: String,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub parent_key: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub slowmode: u32,
    #[serde(default)]
    pub user_limit: Option<u32>,
    #[serde(default)]
    pub nsfw: bool,
    #[serde(default)]
    pub private: bool,
    /// Accès par rôle : `{role_id, mode}` avec mode dans
    /// denied | read | write | moderate.
    #[serde(default)]
    pub access: Vec<AccessRuleRequest>,
}

#[derive(Debug, Deserialize)]
pub struct AccessRuleRequest {
    pub role_id: String,
    pub mode: String,
}

#[derive(Debug, Deserialize)]
pub struct ApplyPlanRequest {
    pub items: Vec<PlanItemRequest>,
}

/// Sort d'un element du plan. `skipped` n'est pas un echec propre a l'element :
/// c'est un salon dont la categorie parente a echoue, et qu'on refuse de creer
/// a la racine du serveur — l'utilisateur ne l'a pas demande la.
#[derive(Debug, Serialize)]
pub struct PlanItemResult {
    pub key: String,
    pub name: String,
    pub kind: String,
    pub status: &'static str,
    pub channel_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApplyPlanResponse {
    pub created: usize,
    pub failed: usize,
    pub skipped: usize,
    pub results: Vec<PlanItemResult>,
}

/// POST /api/guild-structure/{guild_id}/apply — cree les salons du plan.
///
/// Le plan est valide EN ENTIER avant le premier appel Discord : une erreur de
/// saisie ne doit pas laisser derriere elle une moitie de structure. Passe ce
/// point, l'execution est best-effort par element (un salon refuse par Discord
/// n'annule pas les precedents, qui sont deja crees et le resteront) et chaque
/// sort est rapporte a l'utilisateur.
pub async fn apply_plan(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<ApplyPlanRequest>,
) -> Result<Json<ApplyPlanResponse>, ApiError> {
    let plan = ChannelPlan {
        items: body
            .items
            .into_iter()
            .map(to_domain)
            .collect::<Result<Vec<_>, _>>()?,
    };
    let ordered = plan.validate_and_order(&guild_id)?;

    // `parent_key` (clé locale au plan) -> ID Discord réellement créé.
    let mut created_ids: HashMap<String, String> = HashMap::new();
    let mut results = Vec::with_capacity(ordered.len());
    let (mut created, mut failed, mut skipped) = (0usize, 0usize, 0usize);

    for item in &ordered {
        let key = item.key.trim().to_string();
        let name = item.normalized_name();

        // Résolution du parent : soit une catégorie du plan (qui doit avoir
        // été créée à l'instant), soit une catégorie déjà sur le serveur.
        let parent_id = match item.parent_key.as_deref().map(str::trim) {
            Some(parent_key) => match created_ids.get(parent_key) {
                Some(id) => Some(id.clone()),
                None => {
                    skipped += 1;
                    results.push(PlanItemResult {
                        key,
                        name,
                        kind: item.kind.as_str().to_string(),
                        status: "skipped",
                        channel_id: None,
                        error: Some("Catégorie parente non créée : salon ignoré.".into()),
                    });
                    continue;
                }
            },
            None => item.parent_id.clone(),
        };

        // Les permissions sont calculées par le domaine, qui seul sait quels
        // bits ont un sens pour ce type de salon.
        let overwrites = overwrites_for(&item.access, item.private, item.kind, &guild_id);
        let spec = NewChannel {
            name: &name,
            kind: item.kind.discord_type(),
            parent_id: parent_id.as_deref(),
            topic: item.topic.as_deref(),
            slowmode: item.slowmode,
            user_limit: item.user_limit,
            nsfw: item.nsfw,
            overwrites: &overwrites,
        };

        match state.discord_api.create_channel(&guild_id, &spec).await {
            Ok(channel_id) => {
                created += 1;
                if item.kind == PlannedChannelKind::Category {
                    created_ids.insert(key.clone(), channel_id.clone());
                }
                results.push(PlanItemResult {
                    key,
                    name,
                    kind: item.kind.as_str().to_string(),
                    status: "created",
                    channel_id: Some(channel_id),
                    error: None,
                });
            }
            Err(e) => {
                failed += 1;
                results.push(PlanItemResult {
                    key,
                    name,
                    kind: item.kind.as_str().to_string(),
                    status: "failed",
                    channel_id: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Visible dans le flux temps réel du panel : une modification de structure
    // faite par un admin ne doit pas être invisible aux autres.
    state.broadcaster.broadcast(
        "guild_structure:applied",
        serde_json::json!({
            "guild_id": guild_id,
            "created": created,
            "failed": failed,
            "skipped": skipped,
        }),
    );

    Ok(Json(ApplyPlanResponse {
        created,
        failed,
        skipped,
        results,
    }))
}

/// DELETE /api/guild-structure/{guild_id}/channels/{channel_id} — supprime un
/// salon existant. Irreversible cote Discord (messages compris) : Owner requis.
///
/// Deux verrous, tous deux indispensables :
///
/// 1. `channel_id` est valide comme snowflake AVANT toute interpolation. La
///    route Discord de suppression est `DELETE /channels/{id}`, sans guild :
///    un identifiant contenant des segments de chemin (`..%2F..%2Fguilds%2F…`)
///    serait normalise par l'URL et viserait un tout autre endpoint, avec le
///    token du bot. Meme regle que `community/announcements.rs`.
/// 2. Le salon doit APPARTENIR a la guild du chemin. Le RBAC autorise sur
///    `guild_id`, mais l'appel Discord, lui, ne connait que le salon : sans
///    cette verification, un owner legitime de la guild A pourrait detruire un
///    salon de la guild B ou le bot est present.
pub async fn delete_channel(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Path((_, channel_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validation::validate_discord_id("channel_id", &channel_id).map_err(ApiError)?;

    let channels = state.discord_api.list_all_channels(&guild_id).await?;
    if !channels.iter().any(|c| c.id == channel_id) {
        return Err(ApiError(DomainError::NotFound(format!(
            "Salon {channel_id} introuvable sur ce serveur."
        ))));
    }

    state.discord_api.delete_channel(&channel_id).await?;
    state.broadcaster.broadcast(
        "guild_structure:channel_deleted",
        serde_json::json!({ "guild_id": guild_id, "channel_id": channel_id }),
    );
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Traduit un item du wire vers le domaine. Les seuls champs qui peuvent
/// echouer ici sont les enumerations (`kind`, `mode`) : le reste est valide par
/// le domaine lui-meme.
fn to_domain(item: PlanItemRequest) -> Result<PlannedChannel, ApiError> {
    let kind = PlannedChannelKind::parse(&item.kind).ok_or_else(|| {
        ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
            format!("Type de salon inconnu : « {} ».", item.kind),
        ))
    })?;
    let access = item
        .access
        .into_iter()
        .map(|a| {
            let mode = AccessMode::parse(&a.mode).ok_or_else(|| {
                ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
                    format!("Niveau d'accès inconnu : « {} ».", a.mode),
                ))
            })?;
            Ok(ChannelAccess {
                role_id: a.role_id,
                mode,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Ok(PlannedChannel {
        key: item.key,
        name: item.name,
        kind,
        parent_key: item.parent_key,
        parent_id: item.parent_id,
        topic: item.topic,
        slowmode: item.slowmode,
        user_limit: item.user_limit,
        nsfw: item.nsfw,
        private: item.private,
        access,
    })
}
