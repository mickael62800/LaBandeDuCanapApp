use super::*;

// ════════════════════════════════════════════════════════════════════════
// JOB 4 : IMAGE CLEANUP
// ════════════════════════════════════════════════════════════════════════

/// Pour chaque template du catalogue, regarde s'il existe encore des
/// serveurs actifs qui utilisent ce template. Si non, et si la derniere
/// activite est plus ancienne que `unused_image_grace_days`, supprime
/// l'image Docker. Docker refusera la suppression si un container l'utilise
/// encore (defense en profondeur).
pub async fn run_image_cleanup(ctx: &JobContext) -> Result<JobReport, DomainError> {
    // Lecture de la config global (defaut sentinel-* sans guild — on prend
    // la premiere guild qui a une config game-portal). Pour rester simple,
    // on prend les defaults via une guild fictive : ils s'appliquent sauf
    // si l'admin a override.
    let cfg = load_game_portal_config(&ctx.bot_config, "_global").await?;
    if !cfg.auto_remove_unused_images {
        return Ok(JobReport {
            job: "image_cleanup",
            processed: 0,
            errors: 0,
            details: serde_json::json!({"skipped": "auto_remove_unused_images=false"}),
        });
    }
    let grace_days = cfg.unused_image_grace_days;
    if grace_days <= 0 {
        return Ok(JobReport {
            job: "image_cleanup",
            processed: 0,
            errors: 0,
            details: serde_json::json!({"skipped": "grace_days <= 0"}),
        });
    }

    let templates = ctx.template_repo.list().await?;
    let template_ids = templates
        .iter()
        .map(|template| template.id)
        .collect::<Vec<_>>();
    let usages = ctx.server_repo.template_usages(&template_ids).await?;
    let now = chrono::Utc::now();
    let mut removed = 0usize;
    let mut errors = 0usize;
    let mut details = serde_json::Map::new();

    for tpl in &templates {
        let Some(usage) = usages.get(&tpl.id) else {
            // Template jamais utilise, image jamais pull -> rien a faire.
            continue;
        };
        if usage.active_count > 0 {
            continue;
        }
        let last = match usage.last_activity_at {
            Some(t) => t,
            None => continue,
        };
        let cutoff = now - chrono::Duration::days(grace_days as i64);
        if last >= cutoff {
            // Activite trop recente, on respecte la grace period.
            continue;
        }

        info!(template = %tpl.slug, image = %tpl.image, days = grace_days, "image cleanup");
        match ctx.container_runtime.remove_image(&tpl.image, false).await {
            Ok(true) => {
                removed += 1;
                details.insert(tpl.slug.clone(), serde_json::json!("removed"));
                let _ = ctx
                    .audit_repo
                    .log(
                        "_global",
                        None,
                        None,
                        crate::domain::entities::game::audit::GameAuditAction::Delete,
                        serde_json::json!({
                            "kind": "image_cleanup",
                            "template": tpl.slug,
                            "image": tpl.image,
                        }),
                    )
                    .await;
            }
            Ok(false) => {
                details.insert(tpl.slug.clone(), serde_json::json!("not_present"));
            }
            Err(e) => {
                warn!(error = %e, template = %tpl.slug, "image_cleanup failed");
                errors += 1;
                details.insert(tpl.slug.clone(), serde_json::json!(format!("error: {e}")));
            }
        }
    }

    Ok(JobReport {
        job: "image_cleanup",
        processed: removed,
        errors,
        details: serde_json::Value::Object(details),
    })
}
