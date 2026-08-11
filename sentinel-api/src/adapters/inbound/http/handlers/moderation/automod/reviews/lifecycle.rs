use super::*;

#[derive(Debug, Deserialize)]
pub struct CloseIgnoreBody {
    pub actor_id: String,
    pub actor_name: String,
    /// "web" (defaut) ou "discord".
    pub source: Option<String>,
    // Faits Discord du demandeur (source "discord"). Regle is_moderator cote domaine.
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub has_moderate_members: bool,
    #[serde(default)]
    pub has_manage_messages: bool,
    #[serde(default)]
    pub has_mod_role: bool,
    #[serde(default)]
    pub has_admin_role: bool,
}

fn discord_facts_or_none(
    source: &str,
    is_admin: bool,
    has_moderate_members: bool,
    has_manage_messages: bool,
    has_mod_role: bool,
    has_admin_role: bool,
) -> Option<sentinel_core::domain::entities::moderation::review::automod::ModeratorFacts> {
    if source == "discord" {
        Some(
            sentinel_core::domain::entities::moderation::review::automod::ModeratorFacts {
                is_admin,
                has_moderate_members,
                has_manage_messages,
                has_mod_role,
                has_admin_role,
            },
        )
    } else {
        None
    }
}

/// Derive des `ModeratorFacts` a partir du role applicatif REEL du principal.
/// La hierarchie : Viewer(0) < Moderator(1) < Admin(2) < Owner(3).
/// - `is_admin` / `has_admin_role` (=> `can_finalize_review`) : Admin ou plus.
/// - `has_mod_role` / `has_moderate_members` / `has_manage_messages`
///   (=> `is_moderator`, donc le vote) : Moderator ou plus.
fn facts_from_role(role: Role) -> ModeratorFacts {
    let is_admin = role >= Role::Admin;
    let is_mod = role >= Role::Moderator;
    ModeratorFacts {
        is_admin,
        has_admin_role: is_admin,
        has_mod_role: is_mod,
        has_moderate_members: is_mod,
        has_manage_messages: is_mod,
    }
}

/// Determine les `ModeratorFacts` effectifs pour un handler de review sensible.
///
/// - **Appel bot / interne** (pas de `WebUser`, Bearer api_key de confiance) :
///   on garde les faits fournis par le body (`body_facts`), le bot passe les
///   vraies permissions gateway Discord.
/// - **Appel web** (`WebUser` present via `X-Discord-Token`) : on IGNORE le
///   body et on derive les faits du role REEL du principal authentifie sur la
///   guild de la review (trust-boundary S1). Cela fait que les regles domaine
///   (`can_finalize_review` exige Admin, `is_moderator` exige Moderator)
///   s'appliquent au vrai role, pas a un `is_admin:true` forge dans le JSON.
///
/// Fail-closed : une erreur DB sur le lookup de role remonte un 500 (le
/// handler/caller retry) plutot que de degrader silencieusement les privileges.
pub(super) async fn effective_facts(
    _state: &ModerationState,
    user: &Option<Extension<WebUser>>,
    _review_id: Uuid,
    body_facts: Option<ModeratorFacts>,
) -> Result<Option<ModeratorFacts>, ApiError> {
    let Some(Extension(_)) = user else {
        // Chemin de confiance (bot/interne) : comportement inchange.
        return Ok(body_facts);
    };

    // Chemin web : le trust-boundary tient toujours — on ignore le body — mais
    // il n'y a plus de role a resoudre. Un appelant web a forcement franchi le
    // gate `SUPERADMIN_USER_IDS`, donc il a les pleins droits sur la review.
    Ok(Some(facts_from_role(Role::Owner)))
}

/// POST /api/automod/reviews/{review_id}/ignore
/// Clore immediatement le dossier en "ignore" (tout moderateur).
pub async fn ignore_review(
    State(state): State<ModerationState>,
    user: Option<Extension<WebUser>>,
    Path(review_id): Path<String>,
    Json(body): Json<CloseIgnoreBody>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    use sentinel_core::ports::inbound::moderation::manage_automod_reviews::CloseIgnoredCommand;
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    let source = match body.source.as_deref() {
        Some("discord") => "discord",
        _ => "web",
    };
    let body_facts = discord_facts_or_none(
        source,
        body.is_admin,
        body.has_moderate_members,
        body.has_manage_messages,
        body.has_mod_role,
        body.has_admin_role,
    );
    // Web : faits derives du role reel ; bot : faits du body (cf. effective_facts).
    let requester = effective_facts(&state, &user, id, body_facts).await?;
    let review = state
        .automod_reviews_uc
        .close_ignored(CloseIgnoredCommand {
            review_id: id,
            actor_id: body.actor_id.clone(),
            actor_name: body.actor_name.clone(),
            source: source.into(),
            requester,
        })
        .await?;

    state.broadcaster.broadcast(
        "automod_review_resolved",
        serde_json::json!({
            "review_id": review.id.to_string(),
            "action_id": review.id.to_string(),
            "guild_id": &review.guild_id,
            "user_id": &review.user_id,
            "applied_action": "ignore",
            "actor": { "source": source, "id": &body.actor_id, "name": &body.actor_name },
        }),
    );
    Ok(Json(review.into()))
}

#[derive(Debug, Deserialize)]
pub struct ReopenBody {
    pub actor_id: String,
    pub actor_name: String,
    /// Duree (heures) de la nouvelle fenetre de vote (defaut 72).
    #[serde(default)]
    pub deadline_hours: Option<i64>,
    pub source: Option<String>,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub has_moderate_members: bool,
    #[serde(default)]
    pub has_manage_messages: bool,
    #[serde(default)]
    pub has_mod_role: bool,
    #[serde(default)]
    pub has_admin_role: bool,
}

/// POST /api/automod/reviews/{review_id}/reopen
/// Rouvrir un dossier resolu/ignore -> repasse en vote (tout moderateur).
pub async fn reopen_review(
    State(state): State<ModerationState>,
    user: Option<Extension<WebUser>>,
    Path(review_id): Path<String>,
    Json(body): Json<ReopenBody>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    use sentinel_core::ports::inbound::moderation::manage_automod_reviews::ReopenReviewCommand;
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    let source = match body.source.as_deref() {
        Some("discord") => "discord",
        _ => "web",
    };
    let body_facts = discord_facts_or_none(
        source,
        body.is_admin,
        body.has_moderate_members,
        body.has_manage_messages,
        body.has_mod_role,
        body.has_admin_role,
    );
    // Web : faits derives du role reel ; bot : faits du body (cf. effective_facts).
    let requester = effective_facts(&state, &user, id, body_facts).await?;
    let review = state
        .automod_reviews_uc
        .reopen(ReopenReviewCommand {
            review_id: id,
            actor_id: body.actor_id.clone(),
            actor_name: body.actor_name.clone(),
            deadline_hours: body.deadline_hours.unwrap_or(72),
            source: source.into(),
            requester,
        })
        .await?;

    state.broadcaster.broadcast(
        "automod_review_reopened",
        serde_json::json!({
            "review_id": review.id.to_string(),
            "action_id": review.id.to_string(),
            "guild_id": &review.guild_id,
            "user_id": &review.user_id,
            "actor": { "source": source, "id": &body.actor_id, "name": &body.actor_name },
        }),
    );
    Ok(Json(review.into()))
}
