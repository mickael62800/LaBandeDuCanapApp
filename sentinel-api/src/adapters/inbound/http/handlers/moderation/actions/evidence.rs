use super::*;

/// MOD #2 — POST /api/moderation/evidence
///
/// Attache une preuve (URL + description optionnelle) a une action de moderation
/// existante. La FK assure qu'on ne peut pas attacher a une action inconnue.
#[derive(Debug, serde::Deserialize)]
pub struct AddEvidenceDto {
    pub action_id: String,
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
    pub uploaded_by: String,
    pub uploaded_by_name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct EvidenceEntryDto {
    pub id: String,
    pub action_id: String,
    pub url: String,
    pub description: Option<String>,
    pub uploaded_by: String,
    pub uploaded_by_name: String,
    pub uploaded_at: String,
}

pub async fn add_evidence(
    State(state): State<ModerationState>,
    user: Option<Extension<WebUser>>,
    Json(dto): Json<AddEvidenceDto>,
) -> Result<Json<EvidenceEntryDto>, ApiError> {
    // Pour gater user on a besoin du guild_id : on le recupere via l'action liee.
    if user.is_some() {
        if let Ok(_action_uuid) = uuid::Uuid::parse_str(&dto.action_id) {}
    }
    // Validation URL — regle metier dans `domain/entities/moderation_review.rs`.
    sentinel_core::domain::entities::moderation::review::manual::validate_evidence_url(&dto.url)
        .map_err(|m| {
            ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
                m.into(),
            ))
        })?;
    let action_uuid = validation::parse_uuid("action_id", &dto.action_id).map_err(ApiError)?;
    validation::validate_discord_id("uploaded_by", &dto.uploaded_by).map_err(ApiError)?;
    let description = dto
        .description
        .as_deref()
        .map(sentinel_core::domain::entities::moderation::review::manual::truncate_review_text);

    let entry = state
        .evidence_repo
        .add(
            action_uuid,
            &dto.url,
            description.as_deref(),
            &dto.uploaded_by,
            &dto.uploaded_by_name,
        )
        .await?;

    Ok(Json(EvidenceEntryDto {
        id: entry.id.to_string(),
        action_id: dto.action_id,
        url: entry.url,
        description: entry.description,
        uploaded_by: dto.uploaded_by,
        uploaded_by_name: dto.uploaded_by_name,
        uploaded_at: entry.uploaded_at.to_rfc3339(),
    }))
}

/// MOD #2 — GET /api/moderation/evidence/{action_id}
///
/// Liste les preuves attachees a une action.
pub async fn list_evidence(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Path(action_id): Path<String>,
) -> Result<Json<Vec<EvidenceEntryDto>>, ApiError> {
    let action_uuid = validation::parse_uuid("action_id", &action_id).map_err(ApiError)?;

    let entries = state.evidence_repo.list(action_uuid).await?;
    let dtos = entries
        .into_iter()
        .map(|e| EvidenceEntryDto {
            id: e.id.to_string(),
            action_id: action_id.clone(),
            url: e.url,
            description: e.description,
            uploaded_by: e.uploaded_by,
            uploaded_by_name: e.uploaded_by_name,
            uploaded_at: e.uploaded_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(dtos))
}
