//! Surface HTTP d'administration d'Atrium.
//!
//! Les routes exposent l'état par serveur, les quotas en lecture, la
//! configuration de ton, la base de connaissances et les opérations mémoire.
//! Elles réutilisent les mêmes stores que gRPC afin que le dashboard et le bot
//! observent les mêmes données.
//!
//! POURQUOI CE MODULE EXISTE
//!
//! Tout le pilotage d'Atrium — etat par serveur, quotas, base de
//! connaissances — n'existait qu'en gRPC, a l'usage exclusif d'`atrium-bot`.
//! Le back-office n'avait donc AUCUN moyen d'afficher ni de modifier quoi que
//! ce soit : la plateforme etait pilotable depuis Discord et invisible depuis
//! le web. Ces routes comblent ce trou, sans dupliquer la logique : elles
//! s'appuient sur les memes stores (`control`, `budget`, `rag`).
//!
//! SECURITE
//!
//! Meme jeton `ATRIUM_API_TOKEN` que le reste de l'API, verifie par le
//! middleware Bearer commun. Le navigateur ne le connait jamais : nginx l'injecte cote
//! serveur sur `/atrium-api/`, apres avoir valide la session Discord et
//! l'appartenance a SUPERADMIN_USER_IDS (`auth_request`). C'est exactement le
//! montage retenu pour Nexus — un seul modele de passerelle a comprendre.
//!
//! Les quotas sont en LECTURE SEULE : ils viennent de la configuration du
//! processus (variables d'environnement), pas de la base. Les rendre editables
//! ici laisserait croire a un reglage par serveur qui n'existe pas.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::atrium::{budget::BudgetStats, guild_config, rag::IndexedDocument, ApiError, AppState};

/// Un identifiant Discord est un entier 64 bits en decimal : au plus 20
/// chiffres, rien d'autre.
///
/// Ces routes recevaient le `guild_id` du chemin sans le regarder. Le SQL est
/// parametre, donc rien n'etait injectable — mais une chaine arbitraire
/// atteignait quand meme la base, et `set_config` pouvait creer des lignes de
/// configuration pour un « serveur » qui n'en est pas un, invisibles ensuite
/// dans l'interface.
fn valider_guild_id(guild_id: &str) -> Result<(), ApiError> {
    valider_snowflake(guild_id, "guild_id invalide")
}

/// Meme regle pour un identifiant de membre : un `member_id` non valide
/// atteindrait la clause `WHERE` d'un DELETE sans jamais correspondre, et
/// l'effacement repondrait « 0 message » au lieu de « cet identifiant n'en est
/// pas un ». Sur une demande d'effacement, cette confusion se paie cher.
fn valider_member_id(member_id: &str) -> Result<(), ApiError> {
    valider_snowflake(member_id, "member_id invalide")
}

fn valider_snowflake(valeur: &str, message: &'static str) -> Result<(), ApiError> {
    if valeur.is_empty() || valeur.len() > 20 || !valeur.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError::bad_request(message));
    }
    Ok(())
}

#[derive(Serialize)]
pub struct StateResponse {
    pub guild_id: String,
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct SetStateRequest {
    pub enabled: bool,
    /// Identifiant Discord de l'administrateur a l'origine du changement,
    /// conserve dans `atrium_guild_settings.updated_by`. Une bascule sans
    /// auteur rend l'historique inexploitable le jour ou l'on se demande qui
    /// a coupe Atrium.
    pub actor_id: String,
}

/// Etat active/desactive d'Atrium pour un serveur.
pub async fn get_state(
    Extension(state): Extension<Arc<AppState>>,
    Path(guild_id): Path<String>,
) -> Result<Json<StateResponse>, ApiError> {
    valider_guild_id(&guild_id)?;
    let control = state
        .control
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("pilotage Atrium indisponible"))?;
    let enabled = control.is_enabled(&guild_id).await.map_err(|error| {
        tracing::error!(%error, "Lecture de l'etat Atrium impossible");
        ApiError::unavailable("lecture de l'etat impossible")
    })?;
    Ok(Json(StateResponse { guild_id, enabled }))
}

pub async fn set_state(
    Extension(state): Extension<Arc<AppState>>,
    Path(guild_id): Path<String>,
    Json(request): Json<SetStateRequest>,
) -> Result<Json<StateResponse>, ApiError> {
    if request.actor_id.trim().is_empty() {
        return Err(ApiError::bad_request("actor_id requis"));
    }
    let control = state
        .control
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("pilotage Atrium indisponible"))?;
    control
        .set_enabled(&guild_id, request.enabled, request.actor_id.trim())
        .await
        .map_err(|error| {
            tracing::error!(%error, "Ecriture de l'etat Atrium impossible");
            ApiError::unavailable("ecriture de l'etat impossible")
        })?;
    tracing::info!(
        guild_id = %guild_id,
        enabled = request.enabled,
        actor = %request.actor_id,
        "Etat Atrium modifie depuis le back-office"
    );
    Ok(Json(StateResponse {
        guild_id,
        enabled: request.enabled,
    }))
}

/// Consommation du jour + limites configurees.
pub async fn get_usage(
    Extension(state): Extension<Arc<AppState>>,
    Path(guild_id): Path<String>,
) -> Result<Json<BudgetStats>, ApiError> {
    valider_guild_id(&guild_id)?;
    let budget = state
        .budget
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("quotas indisponibles"))?;
    let stats = budget.stats(&guild_id).await.map_err(|error| {
        tracing::error!(%error, "Lecture des quotas Atrium impossible");
        ApiError::unavailable("lecture des quotas impossible")
    })?;
    Ok(Json(stats))
}

#[derive(Serialize)]
pub struct ContextConfigResponse {
    pub welcome_context: String,
    pub conflict_context: String,
    /// Fenetre de depart eclair, en minutes. Renvoyee en chaine comme les
    /// autres cles brutes : le formulaire edite du texte, et une valeur absente
    /// doit s'afficher comme le defaut du schema, pas comme 0.
    pub welcome_ghost_minutes: String,
}

/// Consignes de ton par serveur, en lecture (préremplissage du formulaire).
///
/// Séparé des quotas : ce sont des textes libres, pas des compteurs, et l'écran
/// les édite dans un bloc distinct.
pub async fn get_config(
    Extension(state): Extension<Arc<AppState>>,
    Path(guild_id): Path<String>,
) -> Result<Json<ContextConfigResponse>, ApiError> {
    valider_guild_id(&guild_id)?;
    let pool = state
        .config_pool
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("configuration indisponible"))?;
    let raw = guild_config::load(pool, &guild_id).await.map_err(|error| {
        tracing::error!(%error, "Lecture de la config Atrium impossible");
        ApiError::unavailable("lecture de la configuration impossible")
    })?;
    Ok(Json(ContextConfigResponse {
        welcome_context: raw.get("welcome_context").cloned().unwrap_or_default(),
        conflict_context: raw.get("conflict_context").cloned().unwrap_or_default(),
        welcome_ghost_minutes: raw
            .get("welcome_ghost_minutes")
            .cloned()
            .unwrap_or_else(|| "30".to_string()),
    }))
}

#[derive(Deserialize)]
pub struct SetConfigRequest {
    /// Cles a ecrire. Une cle absente n'est pas touchee ; une valeur vide
    /// serait ecrite telle quelle, donc le front n'envoie que ce qu'il edite.
    pub values: std::collections::HashMap<String, String>,
}

/// Reglages par serveur, en ecriture.
///
/// Seules les cles declarees dans `bot_definitions.config_schema` sont
/// acceptees : sans ce filtre, n'importe quelle cle pourrait etre inseree dans
/// `bot_guild_config`, ou elle resterait invisible et sans effet — une
/// configuration fantome que personne ne saurait expliquer plus tard.
pub async fn set_config(
    Extension(state): Extension<Arc<AppState>>,
    Path(guild_id): Path<String>,
    Json(request): Json<SetConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    valider_guild_id(&guild_id)?;
    let pool = state
        .config_pool
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("configuration indisponible"))?;

    const ALLOWED: [&str; 7] = [
        "enabled",
        "user_daily_limit",
        "user_cooldown_secs",
        "global_daily_limit",
        "welcome_context",
        "conflict_context",
        "welcome_ghost_minutes",
    ];
    // Clés numériques : bornées, positives. Les autres sont du texte libre
    // (`enabled` est un booléen, `*_context` des consignes de ton) : les passer
    // au parseur entier les rejetterait à tort.
    const NUMERIC: [&str; 4] = [
        "user_daily_limit",
        "user_cooldown_secs",
        "global_daily_limit",
        "welcome_ghost_minutes",
    ];
    // Bornes des textes libres, alignées sur la validation du domaine
    // (`WelcomeError`/`CalmingError` : 2 000 caractères).
    const TEXT_MAX_CHARS: usize = 2_000;

    for (key, value) in &request.values {
        if !ALLOWED.contains(&key.as_str()) {
            return Err(ApiError::bad_request("cle de configuration inconnue"));
        }
        // Les bornes du schema sont declaratives cote formulaire ; l'API refait
        // le controle, car un appel direct ne passe pas par le formulaire.
        if NUMERIC.contains(&key.as_str())
            && value.trim().parse::<i64>().map(|n| n < 0).unwrap_or(true)
        {
            return Err(ApiError::bad_request(
                "valeur numerique attendue, positive ou nulle",
            ));
        }
        if key.ends_with("_context") && value.chars().count() > TEXT_MAX_CHARS {
            return Err(ApiError::bad_request(
                "consigne trop longue (2000 caracteres maximum)",
            ));
        }
        // Les consignes de ton gardent leur mise en forme ; seules les valeurs
        // numeriques et le booleen sont normalises par `trim`.
        let stored = if key.ends_with("_context") {
            value.as_str()
        } else {
            value.trim()
        };
        guild_config::set(pool, &guild_id, key, stored)
            .await
            .map_err(|error| {
                tracing::error!(%error, "Ecriture de la config Atrium impossible");
                ApiError::unavailable("ecriture de la configuration impossible")
            })?;
    }

    tracing::info!(
        guild_id = %guild_id,
        cles = request.values.len(),
        "Configuration Atrium modifiee depuis le back-office"
    );
    Ok(Json(serde_json::json!({ "updated": request.values.len() })))
}

/// Documents indexes dans la base de connaissances de la guilde.
pub async fn get_knowledge(
    Extension(state): Extension<Arc<AppState>>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<IndexedDocument>>, ApiError> {
    valider_guild_id(&guild_id)?;
    let rag = state
        .rag
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("base de connaissances indisponible"))?;
    let documents = rag.documents(&guild_id).await.map_err(|error| {
        tracing::error!(%error, "Lecture des documents Atrium impossible");
        ApiError::unavailable("lecture des documents impossible")
    })?;
    Ok(Json(documents))
}

#[derive(Deserialize)]
pub struct ForgetMemberRequest {
    /// Qui demande l'effacement. Exige comme pour `set_state` : un effacement
    /// sans trace de son auteur pose le meme probleme qu'une bascule d'etat
    /// anonyme — on constate que des donnees ont disparu, sans savoir sur
    /// quelle demande.
    pub actor_id: String,
}

#[derive(Serialize)]
pub struct ForgetMemberResponse {
    pub guild_id: String,
    pub member_id: String,
    /// Messages reellement supprimes. `0` est une reponse valable : le membre
    /// n'avait rien dit, ou l'effacement a deja ete fait.
    pub deleted: u64,
}

/// Efface tout ce qu'Atrium a retenu d'un membre, sur demande.
///
/// La capacite existait dans `memory.rs` mais n'etait exposee NULLE PART : ni
/// route, ni commande. Le compilateur ne dit rien d'une methode publique sans
/// appelant, clippy non plus. Repondre a une demande d'effacement supposait
/// donc un `DELETE` manuel dans Postgres — c'est-a-dire, en pratique, que la
/// demande restait sans suite.
///
/// A ne pas confondre avec la purge des 90 jours (`job_retention`) : celle-ci
/// traite la RETENTION, pas l'effacement sur demande. L'une court toute seule,
/// l'autre repond a une personne.
pub async fn forget_member(
    Extension(state): Extension<Arc<AppState>>,
    Path((guild_id, member_id)): Path<(String, String)>,
    Json(request): Json<ForgetMemberRequest>,
) -> Result<Json<ForgetMemberResponse>, ApiError> {
    valider_guild_id(&guild_id)?;
    valider_member_id(&member_id)?;
    let actor_id = request.actor_id.trim();
    if actor_id.is_empty() {
        return Err(ApiError::bad_request("actor_id requis"));
    }

    let memory = state
        .memory
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("memoire Atrium indisponible"))?;
    let deleted = memory
        .forget_member(&guild_id, &member_id)
        .await
        .map_err(|error| {
            tracing::error!(%error, "Effacement de la memoire du membre impossible");
            ApiError::unavailable("effacement impossible")
        })?;

    // Journalise APRES coup et au niveau `info` : c'est une action
    // irreversible, et la seule trace qui en restera.
    tracing::info!(
        guild_id = %guild_id,
        member_id = %member_id,
        actor = %actor_id,
        deleted,
        "Memoire d'un membre effacee depuis le back-office"
    );

    Ok(Json(ForgetMemberResponse {
        guild_id,
        member_id,
        deleted,
    }))
}

#[derive(Serialize)]
pub struct JobSummaryResponse {
    pub summary: String,
    pub generated_by_ai: bool,
}

/// Endpoint interne/admin declenche par platform-scheduler pour generer la meteo d'ambiance.
pub async fn job_generate_summary(
    Extension(state): Extension<Arc<AppState>>,
    Path(guild_id): Path<String>,
) -> Result<Json<JobSummaryResponse>, ApiError> {
    let pool = state
        .config_pool
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("base Atrium indisponible"))?;
    match crate::shared::job_lock::run(pool, "atrium:server-summary", || {
        run_summary(&state, &guild_id)
    })
    .await
    {
        Ok(Some(response)) => Ok(Json(response)),
        Ok(None) => Err(ApiError::conflict("job deja actif")),
        Err(error) => {
            tracing::error!(%error, "job Atrium en echec");
            Err(ApiError::unavailable("job Atrium en echec"))
        }
    }
}

async fn run_summary(state: &AppState, guild_id: &str) -> Result<JobSummaryResponse, String> {
    valider_guild_id(guild_id).map_err(|_| "guild_id invalide".to_owned())?;
    let memory = state
        .memory
        .as_ref()
        .ok_or_else(|| "memoire Atrium indisponible".to_owned())?;
    let activity = memory
        .get_recent_activity(guild_id, 50)
        .await
        .map_err(|error| {
            tracing::error!(%error, "Lecture de l'activite recente impossible");
            "lecture de l'activite impossible".to_owned()
        })?;

    let reply = state
        .summary
        .generate_summary(platform_core::atrium::domain::ServerSummaryRequest {
            guild_id: guild_id.to_owned(),
            sample_activity: activity,
        })
        .await
        .map_err(|_| "erreur generation du resume".to_owned())?;

    memory
        .save_summary(guild_id, &reply.content)
        .await
        .map_err(|error| {
            tracing::error!(%error, "Sauvegarde du resume meteo impossible");
            "sauvegarde du resume impossible".to_owned()
        })?;

    tracing::info!(guild_id = %guild_id, "Resume meteo Atrium genere et sauvegarde avec succes");
    Ok(JobSummaryResponse {
        summary: reply.content,
        generated_by_ai: reply.generated_by_ai,
    })
}

#[derive(Serialize)]
pub struct JobRetentionResponse {
    pub ok: bool,
}

/// Endpoint interne declenche quotidiennement par platform-scheduler pour purger les
/// vieux compteurs de quota. La purge a ete sortie du chemin critique de
/// `check_and_record` : elle n'a plus a rallonger la transaction verrouillant
/// les compteurs a chaque appel IA. Sans guilde : c'est une operation de
/// maintenance sur toutes les lignes expirees.
pub async fn job_retention(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<JobRetentionResponse>, ApiError> {
    let pool = state
        .config_pool
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("base Atrium indisponible"))?;
    match crate::shared::job_lock::run(pool, "atrium:retention", || run_retention(&state)).await {
        Ok(Some(response)) => Ok(Json(response)),
        Ok(None) => Err(ApiError::conflict("job deja actif")),
        Err(error) => {
            tracing::error!(%error, "job Atrium en echec");
            Err(ApiError::unavailable("job Atrium en echec"))
        }
    }
}

async fn run_retention(state: &AppState) -> Result<JobRetentionResponse, String> {
    let budget = state
        .budget
        .as_ref()
        .ok_or_else(|| "quotas indisponibles".to_owned())?;
    budget.purge_old().await.map_err(|error| {
        tracing::error!(%error, "Purge des quotas Atrium impossible");
        "purge des quotas impossible".to_owned()
    })?;

    // La memoire conversationnelle passe par la meme purge quotidienne : ce sont
    // des propos de membres, et rien ne les effaçait — `remember_exchange` ne
    // borne que le nombre de messages par personne, pas leur duree de vie.
    if let Some(memory) = state.memory.as_ref() {
        let jours = std::env::var("ATRIUM_MEMORY_RETENTION_DAYS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(90);
        match memory.purge_old(jours).await {
            Ok((messages, resumes)) => tracing::info!(
                messages,
                resumes,
                jours,
                "Purge de la memoire conversationnelle Atrium effectuee"
            ),
            Err(error) => {
                tracing::error!(%error, "Purge de la memoire Atrium impossible");
                return Err("purge de la memoire impossible".to_owned());
            }
        }
    }

    tracing::info!("Purge des compteurs de quota Atrium effectuee");
    Ok(JobRetentionResponse { ok: true })
}
