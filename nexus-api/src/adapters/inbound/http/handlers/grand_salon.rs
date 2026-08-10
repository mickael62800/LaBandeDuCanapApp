use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Duration, Utc};
use nexus_core::domain::entities::grand_salon::{
    Cercle, CercleKind, Dossier, GazetteArticle, Habitué, MotionDuSalon, MotionStatus,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ApiError;
use crate::bootstrap::AppState;

#[derive(Deserialize)]
pub struct JoinRequest {
    pub display_name: String,
}
#[derive(Deserialize)]
pub struct MotionRequest {
    pub user_id: String,
    pub titre: String,
    pub texte: String,
    #[serde(default = "default_hours")]
    pub duration_hours: i64,
}
fn default_hours() -> i64 {
    48
}
#[derive(Deserialize)]
pub struct VoteRequest {
    pub user_id: String,
    pub choice: bool,
}
#[derive(Deserialize)]
pub struct CercleRequest {
    pub user_id: String,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub devise: String,
}
#[derive(Deserialize)]
pub struct DossierRequest {
    pub user_id: String,
    pub subject: String,
}
#[derive(Deserialize)]
pub struct RevealRequest {
    pub user_id: String,
}

#[derive(Serialize)]
pub struct HabitueDto {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub display_name: String,
    pub rayonnement: i64,
    pub jetons: i64,
    pub reputation: i64,
    pub bons_plans: i64,
    pub reseau: i64,
    pub joined_at: String,
}
impl From<Habitué> for HabitueDto {
    fn from(h: Habitué) -> Self {
        Self {
            id: h.id,
            guild_id: h.guild_id,
            user_id: h.user_id,
            display_name: h.display_name,
            rayonnement: h.ressources.rayonnement,
            jetons: h.ressources.jetons,
            reputation: h.ressources.reputation,
            bons_plans: h.ressources.bons_plans,
            reseau: h.ressources.reseau,
            joined_at: h.joined_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
pub struct MotionDto {
    pub id: Uuid,
    pub titre: String,
    pub texte: String,
    pub status: &'static str,
    pub closes_at: String,
    pub soutien_pour: i64,
    pub soutien_contre: i64,
}
impl From<MotionDuSalon> for MotionDto {
    fn from(m: MotionDuSalon) -> Self {
        Self {
            id: m.id,
            titre: m.titre,
            texte: m.texte,
            status: match m.status {
                MotionStatus::EnVote => "en_vote",
                MotionStatus::Adoptee => "adoptee",
                MotionStatus::Rejetee => "rejetee",
            },
            closes_at: m.closes_at.to_rfc3339(),
            soutien_pour: m.soutien_pour,
            soutien_contre: m.soutien_contre,
        }
    }
}

#[derive(Serialize)]
pub struct ArticleDto {
    pub id: Uuid,
    pub headline: String,
    pub body: String,
    pub published_at: String,
}
impl From<GazetteArticle> for ArticleDto {
    fn from(a: GazetteArticle) -> Self {
        Self {
            id: a.id,
            headline: a.headline,
            body: a.body,
            published_at: a.published_at.to_rfc3339(),
        }
    }
}

pub async fn join(
    State(s): State<AppState>,
    Path((g, u)): Path<(String, String)>,
    Json(r): Json<JoinRequest>,
) -> Result<Json<HabitueDto>, ApiError> {
    Ok(Json(
        s.grand_salon
            .join(&g, &u, &r.display_name, Utc::now())
            .await?
            .into(),
    ))
}
pub async fn daily(
    State(s): State<AppState>,
    Path((g, u)): Path<(String, String)>,
) -> Result<Json<HabitueDto>, ApiError> {
    Ok(Json(s.grand_salon.daily(&g, &u).await?.into()))
}
pub async fn profile(
    State(s): State<AppState>,
    Path((g, u)): Path<(String, String)>,
) -> Result<Json<HabitueDto>, ApiError> {
    Ok(Json(s.grand_salon.profile(&g, &u).await?.into()))
}
pub async fn motions(
    State(s): State<AppState>,
    Path(g): Path<String>,
) -> Result<Json<Vec<MotionDto>>, ApiError> {
    Ok(Json(
        s.grand_salon
            .motions(&g)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    ))
}
pub async fn propose(
    State(s): State<AppState>,
    Path(g): Path<String>,
    Json(r): Json<MotionRequest>,
) -> Result<Json<MotionDto>, ApiError> {
    let author = s.grand_salon.profile(&g, &r.user_id).await?;
    let m = MotionDuSalon {
        id: Uuid::new_v4(),
        guild_id: g,
        titre: r.titre.trim().into(),
        texte: r.texte.trim().into(),
        status: MotionStatus::EnVote,
        author_id: author.id,
        closes_at: Utc::now() + Duration::hours(r.duration_hours.clamp(1, 168)),
        soutien_pour: 0,
        soutien_contre: 0,
    };
    s.grand_salon.propose_motion(m.clone()).await?;
    Ok(Json(m.into()))
}
pub async fn vote(
    State(s): State<AppState>,
    Path((g, id)): Path<(String, Uuid)>,
    Json(r): Json<VoteRequest>,
) -> Result<(), ApiError> {
    s.grand_salon.vote(&g, &r.user_id, id, r.choice).await?;
    Ok(())
}
pub async fn gazette(
    State(s): State<AppState>,
    Path(g): Path<String>,
) -> Result<Json<Vec<ArticleDto>>, ApiError> {
    Ok(Json(
        s.grand_salon
            .gazette(&g)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    ))
}

#[derive(Serialize)]
pub struct CloseReport {
    pub processed: usize,
    pub errors: usize,
}

pub async fn close_due(State(s): State<AppState>) -> Result<Json<CloseReport>, ApiError> {
    let processed = s.grand_salon.close_due_motions(&[], Utc::now()).await?;
    Ok(Json(CloseReport {
        processed,
        errors: 0,
    }))
}

#[derive(Serialize)]
pub struct CercleDto {
    pub id: Uuid,
    pub kind: &'static str,
    pub name: String,
    pub devise: String,
    pub caisse: i64,
    pub reputation: i64,
    pub rayonnement: i64,
}
impl From<Cercle> for CercleDto {
    fn from(c: Cercle) -> Self {
        Self {
            id: c.id,
            kind: match c.kind {
                CercleKind::Bande => "bande",
                CercleKind::Club => "club",
                CercleKind::Collectif => "collectif",
            },
            name: c.name,
            devise: c.devise,
            caisse: c.caisse,
            reputation: c.reputation,
            rayonnement: c.rayonnement,
        }
    }
}
#[derive(Serialize)]
pub struct DossierDto {
    pub id: Uuid,
    pub subject: String,
    pub verified: bool,
    pub revealed_at: Option<String>,
}
impl From<Dossier> for DossierDto {
    fn from(d: Dossier) -> Self {
        Self {
            id: d.id,
            subject: d.subject,
            verified: d.verified,
            revealed_at: d.revealed_at.map(|v| v.to_rfc3339()),
        }
    }
}

pub async fn cercles(
    State(s): State<AppState>,
    Path(g): Path<String>,
) -> Result<Json<Vec<CercleDto>>, ApiError> {
    Ok(Json(
        s.grand_salon
            .cercles(&g)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    ))
}
pub async fn create_cercle(
    State(s): State<AppState>,
    Path(g): Path<String>,
    Json(r): Json<CercleRequest>,
) -> Result<Json<CercleDto>, ApiError> {
    let kind = match r.kind.as_str() {
        "bande" => CercleKind::Bande,
        "club" => CercleKind::Club,
        "collectif" => CercleKind::Collectif,
        _ => {
            return Err(nexus_core::domain::errors::DomainError::ValidationError(
                "type de cercle invalide".into(),
            )
            .into())
        }
    };
    Ok(Json(
        s.grand_salon
            .create_cercle(&g, &r.user_id, kind, &r.name, &r.devise, Utc::now())
            .await?
            .into(),
    ))
}
pub async fn dossiers(
    State(s): State<AppState>,
    Path((g, u)): Path<(String, String)>,
) -> Result<Json<Vec<DossierDto>>, ApiError> {
    Ok(Json(
        s.grand_salon
            .dossiers(&g, &u)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    ))
}
pub async fn investigate(
    State(s): State<AppState>,
    Path(g): Path<String>,
    Json(r): Json<DossierRequest>,
) -> Result<Json<DossierDto>, ApiError> {
    Ok(Json(
        s.grand_salon
            .investigate(&g, &r.user_id, &r.subject)
            .await?
            .into(),
    ))
}
pub async fn reveal(
    State(s): State<AppState>,
    Path((g, id)): Path<(String, Uuid)>,
    Json(r): Json<RevealRequest>,
) -> Result<(), ApiError> {
    s.grand_salon.reveal(&g, &r.user_id, id, Utc::now()).await?;
    Ok(())
}
