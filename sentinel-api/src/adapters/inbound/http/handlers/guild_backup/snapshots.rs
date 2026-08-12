//! Handlers HTTP de sauvegarde / restauration de serveur (`guild_backup`).
//!
//! Action PUISSANTE : la restauration avec `wipe` supprime tous les salons,
//! roles et emojis du serveur avant de les recreer.
//!
//! **Controle d'acces** : `auth_middleware` puis `superadmin_middleware`, poses
//! au niveau du routeur. Il n'y a plus de role « Owner » — le RBAC multi-roles
//! a ete supprime (migration 007). Les appels internes (bot/worker, Bearer
//! `API_KEY`, sans `X-Discord-Token`) passent en tant que service de confiance.
//!
//! L'API ne touche pas a Discord : elle stocke, et publie un event Redis que le
//! bot execute. Cet event est **signe** (cf. `http::event_signing`), le bus
//! etant commun aux trois plateformes.

use axum::extract::{Path, State};
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::event_signing;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::adapters::inbound::http::validation;
use crate::bootstrap::state::GuildBackupState;
use axum::http::StatusCode;
use sentinel_core::domain::entities::guild_backup::snapshot::GuildSnapshot;
use sentinel_core::ports::inbound::guild_backup::manage_snapshots::{SnapshotId, SnapshotSummary};

#[derive(Debug, Serialize)]
pub struct StoredSnapshotDto {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct SnapshotSummaryDto {
    pub id: String,
    pub guild_id: String,
    pub label: String,
    pub created_at: String,
    pub created_by: Option<String>,
    pub schema_version: u32,
    pub role_count: u32,
    pub channel_count: u32,
}

impl From<SnapshotSummary> for SnapshotSummaryDto {
    fn from(s: SnapshotSummary) -> Self {
        SnapshotSummaryDto {
            id: s.id.to_string(),
            guild_id: s.guild_id,
            label: s.label,
            created_at: s.created_at,
            created_by: s.created_by,
            schema_version: s.schema_version,
            role_count: s.role_count,
            channel_count: s.channel_count,
        }
    }
}

/// POST /api/guild-backup/{guild_id}/snapshots — stocke une nouvelle capture.
/// Body = `GuildSnapshot`.
pub async fn store_snapshot(
    State(state): State<GuildBackupState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(mut snapshot): Json<GuildSnapshot>,
) -> Result<(StatusCode, Json<StoredSnapshotDto>), ApiError> {
    // Le guild_id autoritaire est celui du path (evite un mismatch body/URL).
    snapshot.guild_id = guild_id.clone();
    // Quota de retention configurable (guild-backup-bot / snapshot_quota).
    // Absent => defaut historique (20). Le service borne a [1, 100].
    let quota = sentinel_core::domain::entities::system::bot_config::cfg_u64(
        &state
            .bot_config_repo
            .get_config(
                &guild_id,
                sentinel_core::domain::entities::system::bot_names::GUILD_BACKUP_BOT,
            )
            .await
            .unwrap_or_default(),
        "snapshot_quota",
        u64::from(
            sentinel_core::application::guild_backup::manage_snapshots_service::MAX_SNAPSHOTS_PER_GUILD,
        ),
    ) as u32;
    let id = state
        .guild_snapshots_uc
        .store_snapshot_with_quota(snapshot, quota)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(StoredSnapshotDto { id: id.to_string() }),
    ))
}

/// GET /api/guild-backup/{guild_id}/snapshots — liste les captures (resumes).
pub async fn list_snapshots(
    State(state): State<GuildBackupState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<SnapshotSummaryDto>>, ApiError> {
    let summaries = state.guild_snapshots_uc.list_snapshots(&guild_id).await?;
    Ok(Json(summaries.into_iter().map(Into::into).collect()))
}

/// GET /api/guild-backup/snapshots/{snapshot_id} — capture complete (pour la
/// restauration).
pub async fn get_snapshot(
    State(state): State<GuildBackupState>,
    Path(snapshot_id): Path<String>,
) -> Result<Json<GuildSnapshot>, ApiError> {
    let id = parse_id(&snapshot_id)?;
    let snapshot = state.guild_snapshots_uc.get_snapshot(id).await?;
    // Le guild_id vient de la ressource chargee (pas du path) : la gate protege
    // contre une lecture cross-serveur.
    Ok(Json(snapshot))
}

/// DELETE /api/guild-backup/snapshots/{snapshot_id} — supprime une capture.
pub async fn delete_snapshot(
    State(state): State<GuildBackupState>,
    Path(snapshot_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&snapshot_id)?;
    // On charge d'abord pour connaitre le guild_id (RBAC) et distinguer 404.
    state.guild_snapshots_uc.get_snapshot(id).await?;
    state.guild_snapshots_uc.delete_snapshot(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct RenameSnapshotBody {
    pub label: String,
}

/// PATCH /api/guild-backup/snapshots/{snapshot_id} — renomme une capture.
/// Owner de la guild concernee requis (bypass interne bot).
pub async fn rename_snapshot(
    State(state): State<GuildBackupState>,
    Path(snapshot_id): Path<String>,
    Json(body): Json<RenameSnapshotBody>,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&snapshot_id)?;
    // On charge d'abord pour connaitre le guild_id (RBAC) et distinguer 404.
    state.guild_snapshots_uc.get_snapshot(id).await?;
    state
        .guild_snapshots_uc
        .rename_snapshot(id, &body.label)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Corps de `POST /{guild_id}/capture` — demande de capture (executee par le bot).
#[derive(Debug, Deserialize)]
pub struct CaptureRequestBody {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub requested_by: Option<String>,
}

/// POST /api/guild-backup/{guild_id}/capture — publie un event Redis pour que
/// le bot capture le serveur. Le web ne peut pas agir sur Discord : l'API se
/// contente de publier `guild_backup:capture_requested`.
///
/// `requested_by` est derive de l'identite AUTHENTIFIEE, jamais du corps :
/// c'est une trace d'audit d'une action massive, un appelant ne doit pas
/// pouvoir l'attribuer a quelqu'un d'autre.
pub async fn request_capture(
    State(state): State<GuildBackupState>,
    user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<CaptureRequestBody>,
) -> Result<StatusCode, ApiError> {
    let sig = event_signing::sign(
        &state.api_key,
        &event_signing::guild_backup_capture_message(&guild_id),
    );
    state.broadcaster.broadcast(
        "guild_backup:capture_requested",
        serde_json::json!({
            "guild_id": guild_id,
            "label": body.label,
            "requested_by": resolve_requester(&user),
            "sig": sig,
        }),
    );
    Ok(StatusCode::ACCEPTED)
}

/// Identite a journaliser pour une action de sauvegarde/restauration.
///
/// Renvoie l'identifiant Discord authentifie quand l'appel vient du web. Pour
/// un appel interne (bot/worker, Bearer sans `X-Discord-Token`), renvoie
/// `"internal"` : mieux vaut une trace explicite qu'un `null` ambigu.
fn resolve_requester(user: &Option<Extension<WebUser>>) -> String {
    match user {
        Some(Extension(ctx)) => ctx.discord_user_id.clone(),
        None => "internal".to_string(),
    }
}

/// Corps de `POST /snapshots/{snapshot_id}/restore` — demande de restauration.
///
/// `requested_by` a ete RETIRE volontairement : il etait fourni par l'appelant
/// et servait ensuite de trace d'audit sur l'operation la plus destructive de
/// l'API. N'importe qui pouvait donc attribuer un restore a un autre membre.
/// L'identite est desormais derivee de l'authentification.
#[derive(Debug, Deserialize)]
pub struct RestoreRequestBody {
    #[serde(default)]
    pub wipe: bool,
}

/// POST /api/guild-backup/snapshots/{snapshot_id}/restore — publie un event
/// Redis pour que le bot restaure le serveur depuis la capture.
///
/// Le reglage « Roles autorises a restaurer » n'a jamais ete applique (cf. la
/// migration qui le retire du schema de configuration).
pub async fn request_restore(
    State(state): State<GuildBackupState>,
    user: Option<Extension<WebUser>>,
    Path(snapshot_id): Path<String>,
    Json(body): Json<RestoreRequestBody>,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&snapshot_id)?;
    // On charge la capture pour resoudre le guild_id (RBAC + payload event).
    let snapshot = state.guild_snapshots_uc.get_snapshot(id).await?;

    // Signature HMAC (secret = API_KEY partagee bot <-> API), meme protection
    // que `guild_reset` : avec `wipe`, cet event fait supprimer TOUS les salons,
    // roles et emojis du serveur. Sans signature, connaitre `REDIS_URL` — ce
    // que font les six bots/workers et la gateway — suffisait a le declencher.
    let sig = event_signing::sign(
        &state.api_key,
        &event_signing::guild_backup_restore_message(
            &snapshot.guild_id,
            &id.to_string(),
            body.wipe,
        ),
    );
    state.broadcaster.broadcast(
        "guild_backup:restore_requested",
        serde_json::json!({
            "guild_id": snapshot.guild_id,
            "snapshot_id": id.to_string(),
            "wipe": body.wipe,
            "requested_by": resolve_requester(&user),
            "sig": sig,
        }),
    );
    Ok(StatusCode::ACCEPTED)
}

fn parse_id(raw: &str) -> Result<SnapshotId, ApiError> {
    validation::parse_uuid("snapshot_id", raw).map_err(ApiError)
}
