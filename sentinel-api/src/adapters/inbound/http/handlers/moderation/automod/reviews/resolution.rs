use super::lifecycle::effective_facts;
use super::*;

#[derive(Debug, Deserialize)]
pub struct ResolveReviewBody {
    /// "warn" | "delete" | "mute" | "ban" | "ignore".
    pub applied_action: String,
    pub resolved_by_id: String,
    pub resolved_by_name: String,
    /// "web" (defaut) ou "discord" (finalisation via bouton admin du bot).
    pub source: Option<String>,
    // Faits Discord du demandeur (source "discord" uniquement). La regle
    // can_finalize_review est appliquee cote domaine.
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

/// Enregistre la sanction de membre correspondant a une resolution de carte,
/// cote serveur (historique de moderation + escalade), au lieu d'un 2e appel
/// HTTP par le bot. Seules les vraies sanctions de membre sont tracees
/// (prevention/warn/mute/ban) ; "delete"/"ignore" ne sont pas des sanctions.
/// Best-effort : un echec est logge mais ne fait pas echouer la resolution.
pub(crate) async fn log_review_sanction(
    moderation_uc: &std::sync::Arc<
        dyn sentinel_core::ports::inbound::moderation::manage_moderation::ManageModerationUseCase,
    >,
    bot_config_repo: &std::sync::Arc<
        dyn sentinel_core::ports::outbound::system::bot_config_repository::BotConfigRepository,
    >,
    broadcaster: &std::sync::Arc<crate::adapters::outbound::ws::broadcaster::EventBroadcaster>,
    review: &AutomodReview,
    applied_action: &str,
    moderator_id: &str,
    moderator_name: &str,
) {
    use sentinel_core::ports::inbound::moderation::manage_moderation::LogModerationCommand;
    use sentinel_core::ports::inbound::moderation::manage_moderation::LoggedModerationAction;

    // La DÉCISION (quelle action journaliser, avec ou sans strike — règles C1
    // anti double-strike et BUG #5 escalade) vit dans le domaine ; le handler
    // n'exécute que les effets (métriques, logs, appels use case).
    use sentinel_core::domain::entities::moderation::review::automod::{
        finalize_sanction_plan, FinalizeSanctionPlan,
    };
    let skip_strike = match finalize_sanction_plan(applied_action, review.sanction_logged) {
        FinalizeSanctionPlan::Nothing => return,
        FinalizeSanctionPlan::AlreadyLogged => {
            metrics::counter!("automod_sanction_log_total", "result" => "skipped_already_logged")
                .increment(1);
            tracing::info!(
                review_id = %review.id,
                action = %applied_action,
                "Sanction déjà journalisée par l'auto-protection : finalisation non re-journalisée (anti double-strike)"
            );
            return;
        }
        FinalizeSanctionPlan::LogWithoutStrike => {
            metrics::counter!("automod_sanction_log_total", "result" => "escalation_no_strike")
                .increment(1);
            tracing::info!(
                review_id = %review.id,
                action = %applied_action,
                "Finalisation plus sévère que l'auto-protection : escalade journalisée sans second strike (BUG #5)"
            );
            true
        }
        FinalizeSanctionPlan::LogWithStrike => false,
    };

    // Duree du mute depuis la config guild (pour le rappel d'expiration + l'historique).
    let duration = if applied_action == "mute" {
        sentinel_core::domain::entities::system::bot_config::cfg_str(
            &bot_config_repo
                .get_config(
                    review.guild_id.as_str(),
                    sentinel_core::domain::entities::system::bot_names::AUTOMOD_BOT,
                )
                .await
                .unwrap_or_default(),
            "mute_duration_secs",
        )
        .and_then(|v| v.parse::<u64>().ok())
    } else {
        None
    };

    let cmd = LogModerationCommand {
        guild_id: review.guild_id.clone(),
        channel_id: review.channel_id.clone(),
        moderator_id: moderator_id.to_string(),
        moderator_name: moderator_name.to_string(),
        target_id: review.user_id.as_str().to_string(),
        target_name: review.user_name.clone(),
        action_type: applied_action.to_string(),
        reason: "Sanction validee via carte AutoMod".to_string(),
        gravity: if applied_action == "warn" {
            Some("medium".to_string())
        } else {
            None
        },
        duration,
    };
    // BUG #5 : en cas d'escalade (skip_strike), on journalise l'action lourde
    // via `log_action` (SANS strike : l'incident a déjà compté son strike lors
    // du mute auto). Sinon, chemin nominal avec strike.
    let logged = if skip_strike {
        match moderation_uc.log_action(cmd).await {
            Ok(action) => LoggedModerationAction {
                action,
                strike: None,
            },
            Err(e) => {
                metrics::counter!("automod_sanction_log_total", "result" => "error").increment(1);
                tracing::error!(error = %e, review_id = %review.id, action = %applied_action, "Echec log escalade sanction (resolve) cote serveur");
                return;
            }
        }
    } else {
        match moderation_uc.log_action_with_strike(cmd).await {
            Ok(l) => l,
            Err(e) => {
                // Compteur "logs manquants" : si non nul en prod, on active l'outbox
                // (cf. ADR / CR revue moderation). Mesure la fenetre resolve->log.
                metrics::counter!("automod_sanction_log_total", "result" => "error").increment(1);
                tracing::error!(error = %e, review_id = %review.id, action = %applied_action, "Echec log sanction (resolve) cote serveur");
                return;
            }
        }
    };
    if !skip_strike {
        metrics::counter!("automod_sanction_log_total", "result" => "ok").increment(1);
    }

    // Memes broadcasts que l'endpoint /api/moderation/actions, pour que le
    // journal web et les notifications de strike restent a jour.
    broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": applied_action,
            "target_id": review.user_id.as_str(),
            "target_name": &review.user_name,
            "moderator_name": moderator_name,
            "reason": "Sanction validee via carte AutoMod",
            "guild_id": review.guild_id.as_str(),
        }),
    );
    if let Some(sr) = &logged.strike {
        if sr.should_trigger_escalation_broadcast() {
            broadcaster.broadcast(
                "strike_added",
                serde_json::json!({
                    "guild_id": review.guild_id.as_str(),
                    "user_id": review.user_id.as_str(),
                    "active_count": sr.active_count,
                    "escalation_action": sr.escalation_action,
                    "escalation_duration": sr.escalation_duration,
                }),
            );
        }
    }
}

/// POST /api/automod/reviews/{review_id}/resolve
///
/// Marque la review comme resolue cote DB et publie l'event
/// `automod.review.resolved` avec `actor.source = "web"` pour que le bot
/// edite la carte Discord (greyed-out + footer "via web") et applique
/// l'action Discord (warn/mute/ban/delete) en miroir.
pub async fn resolve_review(
    State(state): State<ModerationState>,
    user: Option<Extension<WebUser>>,
    Path(review_id): Path<String>,
    Json(body): Json<ResolveReviewBody>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;

    let source = match body.source.as_deref() {
        Some("discord") => "discord",
        _ => "web",
    };
    // Chemin bot/Discord (de confiance) : les faits du body sont les vraies
    // permissions gateway, utilisees seulement pour la finalisation Discord.
    let body_facts = if source == "discord" {
        Some(ModeratorFacts {
            is_admin: body.is_admin,
            has_moderate_members: body.has_moderate_members,
            has_manage_messages: body.has_manage_messages,
            has_mod_role: body.has_mod_role,
            has_admin_role: body.has_admin_role,
        })
    } else {
        None
    };
    // Chemin web (WebUser present) : on IGNORE le body et on derive les
    // faits du role REEL -> `can_finalize_review` exige desormais un vrai Admin.
    let requester = effective_facts(&state, &user, id, body_facts).await?;
    let review = state
        .automod_reviews_uc
        .resolve(ResolveAutomodReviewCommand {
            review_id: id,
            applied_action: body.applied_action.clone(),
            resolved_by_id: body.resolved_by_id.clone(),
            resolved_by_name: body.resolved_by_name.clone(),
            resolved_source: source.into(),
            requester,
        })
        .await?;

    // Tracabilite : on enregistre la sanction de membre cote serveur, dans la
    // meme requete que la resolution (le bot n'a plus a faire un 2e appel
    // HTTP -> plus de fenetre "resolu mais non logge" cote bot).
    log_review_sanction(
        &state.moderation_uc,
        &state.bot_config_repo,
        &state.broadcaster,
        &review,
        &body.applied_action,
        &body.resolved_by_id,
        &body.resolved_by_name,
    )
    .await;

    // Event WebSocket + Redis Stream pour le bot listener.
    state.broadcaster.broadcast(
        "automod_review_resolved",
        serde_json::json!({
            "review_id": review.id.to_string(),
            "action_id": review.id.to_string(),
            "guild_id": &review.guild_id,
            "user_id": &review.user_id,
            "applied_action": &body.applied_action,
            "actor": {
                "source": source,
                "id": &body.resolved_by_id,
                "name": &body.resolved_by_name,
            },
        }),
    );

    Ok(Json(review.into()))
}
