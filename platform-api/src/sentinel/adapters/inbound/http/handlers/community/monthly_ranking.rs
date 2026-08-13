//! Publication du classement mensuel d'activite (texte / vocal / global) sur
//! Discord. Adaptateur ENTRANT mince : RBAC + parse + envoi Discord. Toute la
//! regle metier (gates, deltas, assemblage des tops, baselines) vit dans
//! `ManageMonthlyRankingUseCase` ; le SQL dans `MonthlyRankingRepository`.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::bootstrap::state::CommunityState;

#[derive(Serialize)]
pub struct MonthlyRankingReport {
    pub guilds_published: usize,
    pub guilds_baselined: usize,
    pub guilds_skipped: usize,
    pub status: &'static str,
}

// ── Publication forcee (commande admin `/classement forcer`) ──

#[derive(Deserialize)]
pub struct ForceRankingRequest {
    pub guild_id: String,
    /// "actuel" (mois en cours, defaut) | "precedent" (mois complet ecoule).
    #[serde(default)]
    pub mois: Option<String>,
}

#[derive(Serialize)]
pub struct RankingEntry {
    pub user_id: String,
    pub xp: i64,
}

#[derive(Serialize)]
pub struct ForceRankingResponse {
    pub period_label: String,
    pub note: Option<String>,
    pub text: Vec<RankingEntry>,
    pub voice: Vec<RankingEntry>,
    pub global: Vec<RankingEntry>,
}

fn map_entries(
    entries: Vec<
        platform_core::sentinel::domain::entities::community::monthly_ranking::RankingEntry,
    >,
) -> Vec<RankingEntry> {
    entries
        .into_iter()
        .map(|e| RankingEntry {
            user_id: e.user_id,
            xp: e.xp,
        })
        .collect()
}

/// POST /api/analytics/force-monthly-ranking
///
/// Publication FORCEE a la demande : bypass les gates. Ne poste PAS sur Discord
/// (contrairement au job auto) : renvoie les donnees au bot qui rend l'embed.
///
/// RBAC : `Admin` sur la guild (pass-through pour les appels bot/internes).
pub async fn force_publish_monthly_ranking(
    State(state): State<CommunityState>,
    Json(req): Json<ForceRankingRequest>,
) -> Result<Json<ForceRankingResponse>, ApiError> {
    let data = state
        .monthly_ranking_uc
        .force_ranking(&req.guild_id, req.mois)
        .await?;

    Ok(Json(ForceRankingResponse {
        period_label: data.period_label,
        note: data.note,
        text: map_entries(data.text),
        voice: map_entries(data.voice),
        global: map_entries(data.global),
    }))
}

/// POST /api/analytics/publish-monthly-ranking
///
/// Job auto (tick worker) : le use case applique les gates + pose les baselines
/// et renvoie le plan des classements a poster ; le handler poste sur Discord et
/// notifie le use case (memorisation de la periode publiee).
pub async fn publish_monthly_ranking_all(
    State(state): State<CommunityState>,
) -> Result<Json<MonthlyRankingReport>, ApiError> {
    let plan = state.monthly_ranking_uc.plan_and_baseline().await?;

    let now = chrono::Utc::now();
    let mut published = 0usize;

    for item in &plan.publications {
        let embed = serde_json::json!({
            "title": format!("\u{1f3c6} Classement de {}", item.period_label),
            "description": "Les membres les plus actifs du mois \u{2014} bravo \u{1f44f}",
            "color": 0xF1C40Fu32,
            "fields": [
                { "name": "\u{1f4dd} Top Texte", "value": item.text_block, "inline": false },
                { "name": "\u{1f399}\u{fe0f} Top Vocal", "value": item.voice_block, "inline": false },
                { "name": "\u{1f3c5} Top Global", "value": item.global_block, "inline": false }
            ],
            "timestamp": now.to_rfc3339(),
        });

        // La validation du salon, l'absence de token et le statut HTTP sont
        // traites par l'adaptateur : ici on ne distingue plus que publie /
        // pas publie. `mark_published` ne doit surtout suivre qu'un succes,
        // sinon un classement rate serait considere comme deja diffuse et ne
        // repasserait jamais.
        match state
            .discord_api
            .send_channel_embed(&item.channel_id, embed)
            .await
        {
            Ok(()) => {
                published += 1;
                let _ = state
                    .monthly_ranking_uc
                    .mark_published(&item.guild_id, &item.period)
                    .await;
            }
            Err(e) => {
                tracing::warn!(error = %e, guild = %item.guild_id, "publish_monthly_ranking: publication echouee");
            }
        }
    }

    Ok(Json(MonthlyRankingReport {
        guilds_published: published,
        guilds_baselined: plan.baselined,
        guilds_skipped: plan.skipped,
        status: "ok",
    }))
}
