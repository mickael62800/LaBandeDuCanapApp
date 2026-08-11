//! Jeux joues depuis le site, sous `/api/me/games/*`.
//!
//! # La regle qui structure tout ce fichier
//!
//! Le joueur est TOUJOURS celui de la session Discord. Aucun `user_id` n'est
//! lu depuis l'URL ni depuis le corps de la requete.
//!
//! nexus-api, lui, prend le joueur dans son chemin
//! (`/api/wheel/{guild_id}/{user_id}/spin`) : c'est adapte a un appelant de
//! confiance comme le bot, mais inexploitable tel quel depuis un navigateur.
//! Exposer ces routes reviendrait a laisser n'importe qui tirer la roue a la
//! place d'un autre, ou vider son portefeuille.
//!
//! # Un seul portefeuille
//!
//! Aucune regle de jeu ici : ces handlers relaient vers les MEMES endpoints
//! que le bot Discord. Le quota quotidien et les mouvements de coins vivent
//! dans nexus-core, une seule fois. Tirer la Roue sur le site consomme donc
//! le tirage du jour sur Discord, et le solde est identique des deux cotes —
//! non par synchronisation, mais parce qu'il n'existe qu'un portefeuille.
//!
//! # Role requis
//!
//! `Member` suffit : ce sont les jeux de la communaute, pas du back-office.
//! Le gate `nexus.access` (defaut Admin) protege la CONSOLE d'administration
//! de la plateforme jeux, pas le droit de jouer.

use axum::extract::{Query, State};
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::errors::DomainError;

const HISTORY_LIMIT: i64 = 15;
const LEADERBOARD_LIMIT: i64 = 10;

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct WalletDto {
    pub username: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
    /// Tirage du jour encore disponible ? Livre avec le portefeuille plutot
    /// que par un appel separe : la page a besoin des deux au meme moment,
    /// et un second aller-retour ferait clignoter le bouton.
    pub can_spin: bool,
}

#[derive(Debug, Serialize)]
pub struct TransactionDto {
    pub id: String,
    pub amount: i64,
    pub balance_after: i64,
    pub source: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct RankDto {
    pub username: String,
    pub coins: i64,
    /// Rang a partir de 1. Calcule ici : deux clients doivent afficher la
    /// meme place, ils ne peuvent pas la deduire chacun de leur cote.
    pub rank: i32,
    /// Vrai pour la ligne du lecteur, pour la mettre en evidence.
    pub is_me: bool,
}

#[derive(Debug, Serialize)]
pub struct SpinDto {
    pub case_key: String,
    pub case_label: String,
    pub payout: i64,
    pub balance_after: i64,
    pub is_memorable: bool,
}

/// Le contexte d'authentification, ou une erreur explicite.
fn require_ctx(user: &Option<Extension<WebUser>>) -> Result<&WebUser, ApiError> {
    user.as_ref()
        .map(|Extension(c)| c)
        .ok_or_else(|| ApiError(DomainError::Forbidden("connexion Discord requise".into())))
}

/// La guilde servie par l'installation.
///
/// Prise dans la configuration et non dans l'URL : l'application est
/// mono-serveur, et laisser le client la choisir rouvrirait la porte que le
/// verrou mono-serveur vient de fermer.
fn guild(state: &AppState) -> Result<&str, ApiError> {
    if state.guild_id.is_empty() {
        return Err(ApiError(DomainError::NotImplemented(
            "aucun serveur configure : les jeux sont indisponibles".into(),
        )));
    }
    Ok(&state.guild_id)
}

fn client(
    state: &AppState,
) -> Result<&crate::adapters::outbound::nexus_games::NexusGamesClient, ApiError> {
    if !state.nexus_games.is_configured() {
        return Err(ApiError(DomainError::NotImplemented(
            "plateforme de jeux non configuree".into(),
        )));
    }
    Ok(&state.nexus_games)
}

/// Pseudo d'affichage, resolu cote serveur.
///
/// Jamais lu depuis la requete : il est enregistre avec chaque mouvement de
/// coins et apparait dans le classement. Le laisser au client permettrait de
/// s'y afficher sous le nom de quelqu'un d'autre.
async fn display_name(_state: &AppState, _guild_id: &str, user_id: &str) -> String {
    user_id.to_string()
}

// ── Coussin Piégé ──

#[derive(Debug, Serialize)]
pub struct CoussinDto {
    pub profile: crate::adapters::outbound::nexus_games::CoussinProfile,
    pub items: Vec<crate::adapters::outbound::nexus_games::CoussinItem>,
    pub combats: Vec<crate::adapters::outbound::nexus_games::CoussinCombat>,
    /// Classement de la guilde, pour situer le joueur.
    pub ranking: Vec<crate::adapters::outbound::nexus_games::CoussinProfile>,
}

/// GET /api/me/games/coussin
///
/// Tout le dossier du joueur en UNE reponse : profil, objets, derniers
/// combats, classement. Quatre appels separes auraient fait apparaitre la
/// page par morceaux, et le classement seul n'a aucun sens sans le profil
/// pour s'y situer.
///
/// Lecture seule. Les actions du jeu restent sur Discord : leur interet
/// tient a la reaction dans le salon.
pub async fn my_coussin(
    State(state): State<AppState>,
    user: Option<Extension<WebUser>>,
) -> Result<Json<CoussinDto>, ApiError> {
    let ctx = require_ctx(&user)?;
    let g = guild(&state)?;
    let c = client(&state)?;
    let uid = &ctx.discord_user_id;

    // Le profil d'abord : c'est lui qui inscrit le joueur au premier appel,
    // et sans lui le reste n'a rien a decrire.
    let nom = display_name(&state, g, uid).await;
    let profile = c.coussin_profile(g, uid, &nom).await?;

    // Les trois autres sont accessoires : un echec les vide sans priver la
    // page du profil, qui est l'essentiel.
    let items = c.coussin_inventory(g, uid).await.unwrap_or_default();
    let combats = c.coussin_combats(g, uid, 10).await.unwrap_or_default();
    let ranking = c.coussin_ranking(g, 10).await.unwrap_or_default();

    Ok(Json(CoussinDto {
        profile,
        items,
        combats,
        ranking,
    }))
}

/// GET /api/me/games/wallet
pub async fn my_wallet(
    State(state): State<AppState>,
    user: Option<Extension<WebUser>>,
) -> Result<Json<WalletDto>, ApiError> {
    let ctx = require_ctx(&user)?;
    let g = guild(&state)?;

    let c = client(&state)?;
    let w = c.wallet(g, &ctx.discord_user_id).await?;

    // Le statut ne doit pas faire echouer le portefeuille : en cas de
    // probleme on suppose le tirage disponible, quitte a ce que le clic soit
    // refuse. L'inverse fermerait le bouton a tort.
    let can_spin = c
        .wheel_status(g, &ctx.discord_user_id)
        .await
        .map(|s| s.can_spin)
        .unwrap_or(true);

    Ok(Json(WalletDto {
        username: w.username,
        coins: w.coins,
        total_earned: w.total_earned,
        total_spent: w.total_spent,
        can_spin,
    }))
}

/// GET /api/me/games/history
pub async fn my_history(
    State(state): State<AppState>,
    user: Option<Extension<WebUser>>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Vec<TransactionDto>>, ApiError> {
    let ctx = require_ctx(&user)?;
    let g = guild(&state)?;

    let txs = client(&state)?
        .history(
            g,
            &ctx.discord_user_id,
            q.limit.unwrap_or(HISTORY_LIMIT).clamp(1, 50),
        )
        .await?;

    Ok(Json(
        txs.into_iter()
            .map(|t| TransactionDto {
                id: t.id,
                amount: t.amount,
                balance_after: t.balance_after,
                source: t.source,
                description: t.description,
                created_at: t.created_at,
            })
            .collect(),
    ))
}

/// GET /api/me/games/leaderboard
pub async fn leaderboard(
    State(state): State<AppState>,
    user: Option<Extension<WebUser>>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Vec<RankDto>>, ApiError> {
    let ctx = require_ctx(&user)?;
    let g = guild(&state)?;

    let wallets = client(&state)?
        .leaderboard(g, q.limit.unwrap_or(LEADERBOARD_LIMIT).clamp(1, 25))
        .await?;

    Ok(Json(
        wallets
            .into_iter()
            .enumerate()
            .map(|(i, w)| RankDto {
                is_me: w.user_id == ctx.discord_user_id,
                username: w.username,
                coins: w.coins,
                rank: i as i32 + 1,
            })
            .collect(),
    ))
}

/// GET /api/me/games/wheel/cases
///
/// Sert au DESSIN de la roue. En cas d'echec, le site retombe sur ses cases
/// par defaut : une roue non dessinee serait pire qu'une roue approximative.
pub async fn wheel_cases(
    State(state): State<AppState>,
    user: Option<Extension<WebUser>>,
) -> Result<Json<crate::adapters::outbound::nexus_games::WheelCases>, ApiError> {
    require_ctx(&user)?;
    let g = guild(&state)?;
    Ok(Json(client(&state)?.wheel_cases(g).await?))
}

/// POST /api/me/games/wheel/spin
///
/// Un tirage par jour et par personne. La regle est arbitree par nexus-core
/// (`try_claim_today`, atomique), pas ici : un controle recopie de ce cote
/// aurait diverge de celui du bot des la premiere evolution.
pub async fn spin_wheel(
    State(state): State<AppState>,
    user: Option<Extension<WebUser>>,
) -> Result<Json<SpinDto>, ApiError> {
    let ctx = require_ctx(&user)?;
    let g = guild(&state)?;

    let nom = display_name(&state, g, &ctx.discord_user_id).await;
    let r = client(&state)?
        .spin_wheel(g, &ctx.discord_user_id, &nom)
        .await?;

    Ok(Json(SpinDto {
        case_key: r.case_key,
        case_label: r.case_label,
        payout: r.payout,
        balance_after: r.balance_after,
        is_memorable: r.is_memorable,
    }))
}
