//! Handlers HTTP Game Portal — version nexus.
//!
//! Difference avec sentinel-api : pas de RBAC/component-gates ici, la seule
//! auth est le Bearer global NEXUS_API_KEY (middleware require_api_key).
//!
//! L'identite de l'acteur (audit) vient de la PASSERELLE pour les appels web
//! (`X-Actor-Id`, pose par nginx depuis `auth_request`), et du payload/query
//! pour les appelants internes. Voir `acteur` : le parametre d'URL etait repris
//! tel quel, ce qui rendait toute action attribuable a n'importe qui.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::nexus::adapters::inbound::http::dto::game::servers::{
    CatalogCommandDto, CreateGameServerDto, GameServerDetailDto, GameServerDto, GameServerStatsDto,
    OnlinePlayerDto, RconCommandDto, RconCommandResponseDto, UpdateConfigDto,
};
use crate::nexus::adapters::inbound::http::handlers::ApiError;
use crate::nexus::bootstrap::AppState;
use platform_core::nexus::domain::entities::game::server::{CreateGameServerCommand, GameServer};
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::events::game_events::{
    IP_REVEAL, SERVER_DELETED, SERVER_SCHEDULED, SERVER_STARTED, SERVER_STOPPED,
    SESSION_CHANNELS_RENAMED,
};

/// POST /api/games/{guild_id}/servers
pub async fn create_server(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<CreateGameServerDto>,
) -> Result<(StatusCode, Json<GameServerDto>), ApiError> {
    let cmd = CreateGameServerCommand {
        guild_id: guild_id.clone(),
        template_slug: dto.template_slug,
        name: dto.name,
        allocated_memory_mb: dto.memory_mb,
        cpu_limit: dto.cpu_limit,
        owner_user_id: dto.owner_user_id,
        initial_config: dto.config,
    };
    let server = state.game_servers_uc.create(cmd).await?;

    // Programme la revelation d'IP : delai fourni, sinon defaut de la guild.
    // 0 jour = pas de revelation programmee.
    let default_days = platform_core::nexus::domain::entities::system::bot_config::cfg_i64(
        &state
            .bot_config_repo
            .get_config(&guild_id, super::GAME_PORTAL_BOT)
            .await
            .unwrap_or_default(),
        "ip_reveal_default_days",
        7,
    ) as i32;
    let days = dto.ip_reveal_days.unwrap_or(default_days).max(0);
    if days > 0 {
        let at = chrono::Utc::now() + chrono::Duration::days(i64::from(days));
        let _ = state
            .game_server_repo
            .set_ip_reveal_at(server.id, Some(at))
            .await;
    }

    let hote = hote_public(&state, &guild_id).await;
    Ok((
        StatusCode::CREATED,
        Json(GameServerDto::from(server).avec_hote(hote.as_deref())),
    ))
}

/// Hote public annonce aux joueurs, lu dans la config game-portal de la guild.
///
/// Le bot le lit deja sous la meme cle pour composer l'adresse au moment de la
/// revelation. On le sert aussi a l'administration, mais sans attendre cette
/// revelation : elle protege l'adresse des JOUEURS, pas des administrateurs,
/// qui ont besoin de tester la connexion avant d'ouvrir la session.
pub(super) async fn hote_public(state: &AppState, guild_id: &str) -> Option<String> {
    let cfg = state
        .bot_config_repo
        .get_config(guild_id, super::GAME_PORTAL_BOT)
        .await
        .unwrap_or_default();
    platform_core::nexus::domain::entities::system::bot_config::cfg_str(&cfg, "session_public_host")
        .filter(|h| !h.trim().is_empty())
        .map(str::to_string)
}

/// GET /api/games/{guild_id}/servers
pub async fn list_servers(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GameServerDto>>, ApiError> {
    let list = state.game_servers_uc.list_for_guild(&guild_id).await?;
    // Un seul appel de config pour toute la liste : l'hote est commun a la guild.
    let hote = hote_public(&state, &guild_id).await;
    Ok(Json(
        list.into_iter()
            .map(|s| GameServerDto::from(s).avec_hote(hote.as_deref()))
            .collect(),
    ))
}

/// GET /api/games/servers/{server_id}
pub async fn get_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<GameServerDetailDto>, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let hote = hote_public(&state, &detail.server.guild_id).await;
    Ok(Json(
        GameServerDetailDto::from(detail).avec_hote(hote.as_deref()),
    ))
}

#[derive(Debug, Deserialize)]
pub struct ActorQuery {
    /// Discord user id de l'acteur (audit), pour les appelants INTERNES
    /// uniquement — le bot, qui connait l'utilisateur ayant lance la commande
    /// et n'a pas de session web. Ignore pour tout ce qui vient de la
    /// passerelle : voir `acteur`.
    pub actor_id: Option<String>,
}

/// Pose par la passerelle nginx, valeur constante. Sa seule presence atteste
/// que la requete vient du web ; l'identite, elle, est dans `X-Actor-Id`.
const EN_TETE_SOURCE: &str = "x-actor-source";
const EN_TETE_ACTEUR: &str = "x-actor-id";

/// Resout l'acteur a journaliser.
///
/// LE PROBLEME QU'ON FERME : `actor_id` etait un parametre d'URL repris tel
/// quel. N'importe quelle action tracee — RCON, arret, suppression de serveur —
/// pouvait donc etre attribuee a quelqu'un d'autre en ajoutant
/// `?actor_id=<autre_personne>`. La tracabilite etait falsifiable par le plus
/// simple des moyens, plus simple encore que de forger un en-tete.
///
/// DEUX APPELANTS, DEUX REGIMES :
///
/// - **Depuis le web** (`X-Actor-Source: gateway`) : l'identite vient de
///   `auth_request`, ecrite par nginx qui ECRASE tout en-tete client. Le
///   parametre d'URL est alors IGNORE — c'est ce qui rend l'attribution non
///   falsifiable. S'il manque quand meme (auth mal configuree), on journalise
///   « inconnu » plutot que de croire l'appelant : une trace vide est
///   preferable a une trace fausse.
/// - **Interne** (bot, worker — porteurs de `NEXUS_API_KEY`, hors passerelle) :
///   le parametre reste lu. Ce sont des processus de confiance, et le bot est
///   le seul a connaitre l'utilisateur Discord qui a clique. Le lui retirer
///   remplacerait une information vraie par le proprietaire du serveur.
///
/// Sans acteur exploitable, repli sur le proprietaire du serveur.
///
/// Note : `nexus-api` n'est pas publie sur l'hote. Un navigateur ne peut
/// l'atteindre QUE par la passerelle, donc jamais par le chemin interne.
async fn acteur(
    state: &AppState,
    headers: &HeaderMap,
    server_id: Uuid,
    depuis_url: Option<&str>,
) -> Result<String, ApiError> {
    if let Some(a) = acteur_propose(headers, depuis_url) {
        return Ok(a);
    }
    let detail = state.game_servers_uc.get(server_id).await?;
    Ok(detail.server.owner_user_id)
}

/// Partie decisive de `acteur`, isolee pour etre testable sans etat : c'est
/// ici que se joue « qui a le droit de nommer l'auteur d'une action ».
/// `None` = aucun acteur exploitable, l'appelant retombe sur le proprietaire.
fn acteur_propose(headers: &HeaderMap, depuis_url: Option<&str>) -> Option<String> {
    let non_vide = |v: &str| {
        let v = v.trim();
        (!v.is_empty()).then(|| v.to_owned())
    };

    if headers.contains_key(EN_TETE_SOURCE) {
        return headers
            .get(EN_TETE_ACTEUR)
            .and_then(|v| v.to_str().ok())
            .and_then(non_vide)
            .or_else(|| Some("inconnu".to_owned()));
    }
    depuis_url.and_then(non_vide)
}

#[cfg(test)]
mod tests_acteur {
    use super::*;

    fn entetes(paires: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in paires {
            h.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                v.parse().unwrap(),
            );
        }
        h
    }

    #[test]
    fn la_passerelle_impose_son_acteur_et_ignore_l_url() {
        let h = entetes(&[("x-actor-source", "gateway"), ("x-actor-id", "111")]);
        // Le parametre d'URL designe quelqu'un d'autre : c'est precisement
        // l'attaque que ce point ferme.
        assert_eq!(acteur_propose(&h, Some("222")).as_deref(), Some("111"));
    }

    #[test]
    fn passerelle_sans_identite_journalise_inconnu_plutot_que_l_url() {
        // nginx n'emet pas un en-tete vide : `X-Actor-Id` peut donc manquer si
        // l'authentification est mal configuree. Ne PAS retomber sur l'URL —
        // ce serait rouvrir le defaut le jour ou l'auth casse.
        let h = entetes(&[("x-actor-source", "gateway")]);
        assert_eq!(acteur_propose(&h, Some("222")).as_deref(), Some("inconnu"));
    }

    #[test]
    fn appelant_interne_peut_nommer_l_acteur() {
        // Le bot connait l'utilisateur Discord qui a lance la commande, et
        // n'atteint pas l'API par la passerelle.
        let h = entetes(&[]);
        assert_eq!(acteur_propose(&h, Some("333")).as_deref(), Some("333"));
    }

    #[test]
    fn sans_rien_d_exploitable_on_retombe_sur_le_proprietaire() {
        assert_eq!(acteur_propose(&entetes(&[]), None), None);
        assert_eq!(acteur_propose(&entetes(&[]), Some("   ")), None);
    }
}

/// Publie un evenement de cycle de vie serveur a destination du bot.
/// `guild_id` est lu avant l'action pour rester disponible apres un delete.
async fn publish_lifecycle(state: &AppState, event: &str, server_id: Uuid, guild_id: &str) {
    state
        .events
        .publish(
            event,
            serde_json::json!({
                "server_id": server_id.to_string(),
                "guild_id": guild_id,
            }),
        )
        .await;
}

/// L'evenement de suppression EMPORTE ce que le bot doit nettoyer.
///
/// `on_deleted`, cote nexus-bot, redemandait le serveur a l'API pour lire les
/// identifiants de ses salons. Mais `find_by_id` filtre `deleted_at IS NULL` :
/// juste apres la suppression la fiche n'existe plus, le bot recevait un 404 et
/// abandonnait des sa premiere ligne, sans un mot. Les salons Discord d'un jeu
/// supprime survivaient donc indefiniment.
///
/// Les identifiants sont lus AVANT la suppression et voyagent dans le message.
/// Le bot n'a plus rien a relire, donc plus rien a rater : la course est
/// supprimee, pas contournee.
async fn publish_server_deleted(state: &AppState, server_id: Uuid, server: &GameServer) {
    let payload =
        platform_core::nexus::ports::outbound::events::game_events::payload_serveur_supprime(
            &server_id.to_string(),
            &server.guild_id,
            server.text_channel_id.as_deref(),
            server.voice_channel_id.as_deref(),
            &server.template_id.to_string(),
        );
    state.events.publish(SERVER_DELETED, payload).await;
}

/// POST /api/games/servers/{server_id}/start
///
/// Démarrage en TÂCHE DE FOND : le pull d'image (jusqu'à ~8 Go) + create + start
/// prennent des minutes. Exécuté dans la requête, le client (web ou bot) coupe à
/// son timeout et ANNULE la requête — ce qui interrompait la création du
/// conteneur et laissait le serveur coincé en `starting`. On répond donc 202 dès
/// que l'ordre est pris ; l'UI suit l'état par polling, et une erreur éventuelle
/// est exposée via `last_error` du serveur.
pub async fn start_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    let guild_id = detail.server.guild_id.clone();
    let bg = state.clone();
    tokio::spawn(async move {
        match bg.game_servers_uc.start(server_id, &actor).await {
            Ok(()) => publish_lifecycle(&bg, SERVER_STARTED, server_id, &guild_id).await,
            Err(e) => {
                tracing::warn!(error = %e, %server_id, "start (tache de fond) a echoue")
            }
        }
    });
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/stop
///
/// En TÂCHE DE FOND comme `start` : `docker stop` laisse au conteneur un délai
/// de grâce (`stop_timeout_secs`, 30 s par défaut) avant kill, ce qui dépasse le
/// timeout client (15 s) et faisait annuler la requête. On répond 204 dès que
/// l'ordre est pris ; l'UI suit l'état par polling.
pub async fn stop_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    let guild_id = detail.server.guild_id.clone();
    let bg = state.clone();
    tokio::spawn(async move {
        match bg.game_servers_uc.stop(server_id, &actor).await {
            Ok(()) => publish_lifecycle(&bg, SERVER_STOPPED, server_id, &guild_id).await,
            Err(e) => tracing::warn!(error = %e, %server_id, "stop (tache de fond) a echoue"),
        }
    });
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/restart
pub async fn restart_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    // Comme `start` : `restart` inclut un `start` (donc un possible pull long).
    // On l'exécute en tâche de fond pour ne pas se faire annuler par le timeout
    // client. L'UI suit l'état par polling.
    let bg = state.clone();
    tokio::spawn(async move {
        if let Err(e) = bg.game_servers_uc.restart(server_id, &actor).await {
            tracing::warn!(error = %e, %server_id, "restart (tache de fond) a echoue");
        }
    });
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/reveal-ip
///
/// Force la revelation avant `ip_reveal_at`. La passerelle Web reserve cette
/// route aux administrateurs Nexus ; l'acteur reste journalise dans l'audit.
pub async fn reveal_ip(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    state.game_servers_uc.reveal_ip(server_id, &actor).await?;
    publish_lifecycle(&state, IP_REVEAL, server_id, &detail.server.guild_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Réponse de `/reveal-ip/request` : le bot y lit le décompte à annoncer.
#[derive(Debug, serde::Serialize)]
pub struct RequestRevealDto {
    pub delay_minutes: i64,
    pub reveal_at: chrono::DateTime<chrono::Utc>,
    pub started: bool,
}

/// POST /api/games/servers/{server_id}/reveal-ip/request
///
/// Flux du bouton « Révéler l'adresse IP » : démarre le serveur si besoin et
/// programme la révélation à `now + reveal_delay_minutes`. Le worker `reveal-ip`
/// publiera l'adresse à l'échéance (événement `game_ip_reveal`). On ne publie
/// PAS `game_server_started` ici : les salons existent déjà (le bouton est sur
/// le panneau) et le conteneur est démarré côté API.
pub async fn request_reveal_ip(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
) -> Result<Json<RequestRevealDto>, ApiError> {
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    let outcome = state
        .game_servers_uc
        .request_ip_reveal(server_id, &actor)
        .await?;

    // Démarrage en TÂCHE DE FOND : le pull d'image + create + start prennent
    // des minutes et dépasseraient le timeout HTTP du client (bot ou web). La
    // révélation est déjà programmée ; le worker fera passer l'état à `running`.
    // Le claim atomique de `start` protège contre un double-démarrage.
    if outcome.started {
        let uc = state.game_servers_uc.clone();
        let actor = actor.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.start(server_id, &actor).await {
                tracing::warn!(error = %e, %server_id, "reveal-request: demarrage en tache de fond echoue");
            }
        });
    }

    Ok(Json(RequestRevealDto {
        delay_minutes: outcome.delay_minutes,
        reveal_at: outcome.reveal_at,
        started: outcome.started,
    }))
}

/// Corps des routes de programmation. `reveal_at` optionnel pour
/// `/reveal-schedule` (None efface la programmation) ; requis pour `/schedule`
/// (une valeur nulle y est refusee par le use case).
#[derive(Debug, Deserialize)]
pub struct ScheduleDto {
    pub reveal_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Heure de fin annoncee. Absente = aucune fin prevue ; un conteneur
    /// arrete est alors annonce ferme, faute de pouvoir promettre une reprise.
    #[serde(default)]
    pub closes_at: Option<chrono::DateTime<chrono::Utc>>,
    pub actor_id: Option<String>,
}

/// POST /api/games/servers/{server_id}/schedule
///
/// Mode « Préparation » : programme l'ouverture sans démarrer le conteneur. Le
/// serveur passe `scheduled` et le bot crée dès maintenant les salons + le
/// panneau d'inscription (événement `game_server_scheduled`). Le worker
/// démarrera le conteneur ~5 min avant l'heure.
pub async fn schedule_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Json(dto): Json<ScheduleDto>,
) -> Result<StatusCode, ApiError> {
    let reveal_at = dto.reveal_at.ok_or_else(|| {
        ApiError::from(
            platform_core::nexus::domain::errors::DomainError::ValidationError(
                "reveal_at requis pour programmer l'ouverture".into(),
            ),
        )
    })?;
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = acteur(&state, &headers, server_id, dto.actor_id.as_deref()).await?;
    state
        .game_servers_uc
        .schedule(server_id, reveal_at, dto.closes_at, &actor)
        .await?;
    publish_lifecycle(&state, SERVER_SCHEDULED, server_id, &detail.server.guild_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/reveal-schedule
///
/// Programme (ou efface avec `reveal_at` nul) l'heure de révélation auto de
/// l'IP sans changer l'état du conteneur. Complète « Lancer maintenant » quand
/// on veut aussi une révélation automatique.
pub async fn set_reveal_schedule(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Json(dto): Json<ScheduleDto>,
) -> Result<StatusCode, ApiError> {
    // Pas de lecture prealable du serveur ici : `acteur` ne la fait que si
    // aucune identite n'est exploitable, et cette route ne publie aucun
    // evenement de cycle de vie (donc pas besoin du `guild_id`).
    let actor = acteur(&state, &headers, server_id, dto.actor_id.as_deref()).await?;
    state
        .game_servers_uc
        .set_reveal_schedule(server_id, dto.reveal_at, &actor)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/games/servers/{server_id}
pub async fn delete_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
) -> Result<StatusCode, ApiError> {
    let detail = state.game_servers_uc.get(server_id).await?;
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    state.game_servers_uc.delete(server_id, &actor).await?;
    publish_server_deleted(&state, server_id, &detail.server).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub lines: Option<u32>,
}

/// GET /api/games/servers/{server_id}/logs?lines=200
pub async fn get_logs(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<Vec<String>>, ApiError> {
    let lines = q.lines.unwrap_or(200).min(1000);
    let logs = state.game_servers_uc.get_logs(server_id, lines).await?;
    Ok(Json(logs))
}

/// GET /api/games/servers/{server_id}/stats
pub async fn get_stats(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<GameServerStatsDto>, ApiError> {
    let stats = state.game_servers_uc.get_stats(server_id).await?;
    let mut dto: GameServerStatsDto = stats.into();

    // Le debit et la latence viennent du dernier passage du controle de sante :
    // les statistiques Docker de l'instant ne donnent que des totaux cumules,
    // dont on ne peut rien tirer sans point de comparaison.
    if let Ok(detail) = state.game_servers_uc.get(server_id).await {
        let serveur = detail.server;
        dto.rcon_latency_ms = serveur.rcon_latency_ms;
        if let Some((rx, tx)) = platform_core::nexus::domain::entities::game::server::debit_reseau(
            serveur.net_rx_bytes,
            serveur.net_tx_bytes,
            serveur.net_sampled_at,
            dto.network_rx_bytes as i64,
            dto.network_tx_bytes as i64,
            chrono::Utc::now(),
        ) {
            dto.network_rx_bytes_per_sec = Some(rx);
            dto.network_tx_bytes_per_sec = Some(tx);
        }
    }

    Ok(Json(dto))
}

/// PUT /api/games/servers/{server_id}/config
pub async fn update_config(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<UpdateConfigDto>,
) -> Result<StatusCode, ApiError> {
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    state
        .game_servers_uc
        .update_config(server_id, dto.config, &actor)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/games/servers/{server_id}/command
pub async fn execute_rcon(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<RconCommandDto>,
) -> Result<Json<RconCommandResponseDto>, ApiError> {
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    let resp = state
        .game_servers_uc
        .execute_rcon(server_id, &dto.command, &actor)
        .await?;
    Ok(Json(RconCommandResponseDto { response: resp }))
}

/// GET /api/games/servers/{server_id}/commands
///
/// Catalogue d'administration du jeu. Les gabarits RCON ne sont pas dans la
/// reponse : le navigateur n'a besoin que des libelles et des parametres.
pub async fn list_commands(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<Vec<platform_core::nexus::domain::entities::game::command::GameCommand>>, ApiError>
{
    Ok(Json(state.game_servers_uc.list_commands(server_id).await?))
}

/// POST /api/games/servers/{server_id}/commands/{command_key}
pub async fn run_catalog_command(
    State(state): State<AppState>,
    Path((server_id, command_key)): Path<(Uuid, String)>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<CatalogCommandDto>,
) -> Result<Json<RconCommandResponseDto>, ApiError> {
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    let params: Vec<(String, String)> = dto.params.into_iter().collect();
    let response = state
        .game_servers_uc
        .run_catalog_command(server_id, &command_key, &params, &actor)
        .await?;
    Ok(Json(RconCommandResponseDto { response }))
}

/// GET /api/games/servers/{server_id}/players/online
///
/// Interroge le serveur de jeu en direct, avec la commande propre a ce jeu.
pub async fn list_online_players(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
) -> Result<Json<Vec<OnlinePlayerDto>>, ApiError> {
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    let players = state
        .game_servers_uc
        .list_online_players(server_id, &actor)
        .await?;
    Ok(Json(
        players
            .into_iter()
            .map(|p| OnlinePlayerDto {
                name: p.name,
                game_player_id: p.game_player_id,
            })
            .collect(),
    ))
}

use axum::response::sse::{Event, Sse};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::Stream;

/// GET /api/games/servers/{server_id}/stream-logs?lines=50
pub async fn stream_logs_sse(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LogsQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let lines = q.lines.unwrap_or(50).min(500);
    let logs = state.game_servers_uc.get_logs(server_id, lines).await?;

    let stream = async_stream::stream! {
        for line in logs {
            yield Ok(Event::default().data(line));
        }
    };

    Ok(Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15))))
}

/// GET /api/games/servers/{server_id}/stream-stats
pub async fn stream_stats_sse(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let stats = state.game_servers_uc.get_stats(server_id).await?;

    let stream = async_stream::stream! {
        if let Ok(json) = serde_json::to_string(&GameServerStatsDto::from(stats)) {
            yield Ok(Event::default().data(json));
        }
    };

    Ok(Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15))))
}
// Handlers HTTP du cycle de vie des serveurs de jeu. Un handler convertit la
// requête en commande et délègue la validité métier à platform_core::nexus.

/// Ressources allouees a un serveur.
#[derive(Debug, serde::Deserialize)]
pub struct UpdateResourcesDto {
    pub memory_mb: i32,
    /// `null` = plafond par defaut de l'adapter.
    #[serde(default)]
    pub cpu_limit: Option<f64>,
}

/// PUT /api/games/servers/{server_id}/resources
///
/// Docker fige memoire et processeur a la creation du conteneur : le
/// changement prend effet au prochain demarrage, qui le reconstruit.
pub async fn update_resources(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<UpdateResourcesDto>,
) -> Result<StatusCode, ApiError> {
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    state
        .game_servers_uc
        .update_resources(server_id, dto.memory_mb, dto.cpu_limit, &actor)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Noms libres des salons de ce serveur. Les trois voyagent ensemble : un
/// champ absent vaut « pas de nom libre », pas « ne change rien ».
#[derive(Debug, serde::Deserialize)]
pub struct UpdateChannelNamesDto {
    #[serde(default)]
    pub channel_name_registration: Option<String>,
    #[serde(default)]
    pub channel_name_private: Option<String>,
    #[serde(default)]
    pub channel_name_voice: Option<String>,
}

/// PUT /api/games/servers/{server_id}/channel-names
pub async fn update_channel_names(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<UpdateChannelNamesDto>,
) -> Result<StatusCode, ApiError> {
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;
    state
        .game_servers_uc
        .update_channel_names(
            server_id,
            dto.channel_name_registration,
            dto.channel_name_private,
            dto.channel_name_voice,
            &actor,
        )
        .await?;

    // Le bot renomme les salons DEJA CREES. Publie apres l'ecriture, jamais
    // avant : il relit la fiche pour calculer les noms, et la lirait sinon dans
    // son etat precedent.
    let detail = state.game_servers_uc.get(server_id).await?;
    publish_lifecycle(
        &state,
        SESSION_CHANNELS_RENAMED,
        server_id,
        &detail.server.guild_id,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

// ── Annonce d'ouverture ──

#[derive(Debug, serde::Serialize)]
pub struct SessionAnnouncementDto {
    pub content: String,
}

/// GET /api/games/servers/{server_id}/announcement
///
/// Le bot demande le texte, le publie, puis marque. Il ne le compose pas :
/// seule l'API sait a quel domaine confier la plume.
///
/// 503 SIGNIFIE « RETENTE PLUS TARD », et le bot doit alors s'abstenir de
/// publier le panneau d'inscription. 422 signifie l'inverse : la demande ne
/// passera jamais, inutile d'y revenir.
pub async fn get_session_announcement(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<SessionAnnouncementDto>, ApiError> {
    use platform_core::nexus::ports::inbound::game::session_announcement::SessionAnnouncementError as E;

    match state.session_announcement_uc.rediger(server_id).await {
        Ok(content) => Ok(Json(SessionAnnouncementDto { content })),
        Err(E::Introuvable(id)) => {
            Err(DomainError::NotFound(format!("game_server {id} introuvable")).into())
        }
        Err(erreur @ (E::RienAAnnoncer | E::AbandonApresPlafond)) => {
            Err(DomainError::ValidationError(erreur.to_string()).into())
        }
        Err(E::Redaction(redaction)) => {
            use platform_core::nexus::ports::outbound::game::announcement_gateway::AnnouncementError;
            match redaction {
                AnnouncementError::Indisponible => Err(DomainError::Infrastructure(
                    "Atrium ne peut pas rediger l'annonce".into(),
                )
                .into()),
                AnnouncementError::Refusee(detail) => {
                    Err(DomainError::ValidationError(detail).into())
                }
            }
        }
        Err(E::Interne(detail)) => Err(DomainError::Internal(detail).into()),
    }
}

/// POST /api/games/servers/{server_id}/announcement/posted
///
/// Marque l'annonce comme publiee. Appelee par le bot DES QUE l'annonce est
/// partie, avant le panneau : un panneau rate se rejoue sans dommage, une
/// annonce publiee deux fois se voit.
pub async fn mark_session_announcement_posted(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .session_announcement_uc
        .marquer_publiee(server_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Alertes de supervision ──

/// Reglages d'alerte tels que l'ecran les voit.
///
/// L'URL du webhook n'y figure PAS : c'est un secret — qui l'a peut ecrire
/// dans le salon. L'ecran apprend seulement qu'un webhook est configure.
#[derive(Debug, serde::Serialize)]
pub struct AlertSettingsDto {
    pub configured: bool,
    pub cpu_threshold: i32,
    pub ram_threshold: i32,
    pub latency_threshold_ms: i32,
}

#[derive(Debug, serde::Deserialize)]
pub struct SaveAlertSettingsDto {
    /// Absent ou vide = on garde le webhook deja enregistre. Sans cela,
    /// l'ecran devrait le redemander a chaque modification de seuil, donc le
    /// connaitre — ce qui reviendrait a le lui renvoyer.
    #[serde(default)]
    pub webhook_url: Option<String>,
    pub cpu_threshold: i32,
    pub ram_threshold: i32,
    pub latency_threshold_ms: i32,
}

/// GET /api/games/servers/{server_id}/alerts
pub async fn get_alert_settings(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<AlertSettingsDto>, ApiError> {
    let config = state.game_alert_repo.find(server_id).await?;
    Ok(Json(match config {
        Some(c) => AlertSettingsDto {
            configured: true,
            cpu_threshold: c.settings.cpu_threshold,
            ram_threshold: c.settings.ram_threshold,
            latency_threshold_ms: c.settings.latency_threshold_ms,
        },
        None => AlertSettingsDto {
            configured: false,
            cpu_threshold: 85,
            ram_threshold: 90,
            latency_threshold_ms: 500,
        },
    }))
}

/// PUT /api/games/servers/{server_id}/alerts
pub async fn save_alert_settings(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<SaveAlertSettingsDto>,
) -> Result<StatusCode, ApiError> {
    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;

    let webhook = match dto.webhook_url.as_deref().map(str::trim) {
        Some(url) if !url.is_empty() => {
            // Un webhook Discord et rien d'autre : cette URL est appelee par le
            // serveur, une adresse arbitraire en ferait un relais de requetes
            // sortantes choisi par le navigateur.
            if !url.starts_with("https://discord.com/api/webhooks/")
                && !url.starts_with("https://discordapp.com/api/webhooks/")
            {
                return Err(
                    platform_core::nexus::domain::errors::DomainError::ValidationError(
                        "l'URL doit etre un webhook Discord".into(),
                    )
                    .into(),
                );
            }
            url.to_string()
        }
        // Conserve l'URL existante : l'ecran ne la connait pas, il ne peut donc
        // pas la renvoyer a chaque modification de seuil.
        _ => match state.game_alert_repo.find(server_id).await? {
            Some(existant) => existant.webhook_url,
            None => {
                return Err(
                    platform_core::nexus::domain::errors::DomainError::ValidationError(
                        "aucun webhook enregistre : fournis-en un".into(),
                    )
                    .into(),
                )
            }
        },
    };

    state
        .game_alert_repo
        .upsert(
            server_id,
            &webhook,
            dto.cpu_threshold.clamp(1, 100),
            dto.ram_threshold.clamp(1, 100),
            dto.latency_threshold_ms.clamp(50, 60_000),
            Some(&actor),
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/games/servers/{server_id}/alerts
pub async fn delete_alert_settings(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.game_alert_repo.delete(server_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Plages d'ouverture recurrentes ──

/// Cles de redemarrage automatique connues des images de jeu.
///
/// Un redemarrage programme n'a de sens que sur un serveur qui tourne en
/// continu. Des qu'il ouvre et ferme chaque jour, il redemarre deja — et le
/// cron risque de tomber hors plage, sur un conteneur eteint, ou pire de le
/// relancer juste apres une fermeture.
const CLES_REDEMARRAGE_AUTO: &[&str] = &[
    "AUTO_REBOOT_ENABLED",
    "RESTART_CRON",
    "RESTART_CRON_EXPRESSION",
];

#[derive(Debug, serde::Serialize)]
pub struct ScheduleRangesDto {
    pub enabled: bool,
    /// `ranges` (plages d'ouverture) ou `restart` (permanence 24/24).
    pub mode: String,
    pub timezone: String,
    pub ranges: Vec<platform_core::nexus::domain::entities::game::schedule::TimeRange>,
    pub warn_minutes: u16,
    /// Prochaine ouverture calculee, pour l'annoncer a l'ecran.
    pub next_opening: Option<String>,
    /// Reglages de redemarrage automatique neutralises par les plages.
    pub disabled_restart_keys: Vec<String>,
    /// Mode permanence : heures entre deux redemarrages.
    pub restart_interval_hours: Option<u8>,
    pub restart_anchor_minute: u8,
    /// Prochain redemarrage calcule, pour l'afficher.
    pub next_restart: Option<String>,
    /// Cadences proposees a l'ecran. Envoyees par le serveur pour que la liste
    /// ne puisse pas diverger de ce que le domaine accepte.
    pub restart_interval_choices: Vec<u8>,
}

#[derive(Debug, serde::Deserialize)]
pub struct SaveScheduleRangesDto {
    pub enabled: bool,
    /// Absent = `ranges` : c'est ce que faisaient tous les appels avant ce
    /// champ, et le seul des deux modes qui ne redemarre rien tout seul.
    #[serde(default)]
    pub mode: Option<String>,
    pub timezone: String,
    pub ranges: Vec<platform_core::nexus::domain::entities::game::schedule::TimeRange>,
    pub warn_minutes: u16,
    #[serde(default)]
    pub restart_interval_hours: Option<u8>,
    #[serde(default)]
    pub restart_anchor_minute: u8,
}

/// GET /api/games/servers/{server_id}/schedule-ranges
pub async fn get_schedule_ranges(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<ScheduleRangesDto>, ApiError> {
    use platform_core::nexus::domain::entities::game::schedule::{
        next_opening, next_restart_at, AutoSchedule, ScheduleMode, RESTART_INTERVALS_HOURS,
    };

    let stored = state.game_schedule_repo.find(server_id).await?;
    let closes_at = state
        .game_servers_uc
        .get(server_id)
        .await
        .ok()
        .and_then(|d| d.server.closes_at);

    let dto = match stored {
        Some(s) => {
            let schedule = AutoSchedule {
                enabled: s.enabled,
                mode: s.mode,
                timezone: s.timezone.clone(),
                ranges: s.ranges.clone(),
                warn_minutes: s.warn_minutes,
                closes_at,
                restart_interval_hours: s.restart_interval_hours,
                restart_anchor_minute: s.restart_anchor_minute,
                last_restart_at: s.last_restart_at,
                last_warned_at: s.last_warned_at,
                last_final_warned_at: s.last_final_warned_at,
            };
            let maintenant = chrono::Utc::now();
            ScheduleRangesDto {
                next_opening: next_opening(&schedule, maintenant).map(|d| d.to_rfc3339()),
                next_restart: next_restart_at(&schedule, maintenant).map(|d| d.to_rfc3339()),
                enabled: s.enabled,
                mode: s.mode.as_str().to_string(),
                timezone: s.timezone,
                ranges: s.ranges,
                warn_minutes: s.warn_minutes,
                disabled_restart_keys: Vec::new(),
                restart_interval_hours: s.restart_interval_hours,
                restart_anchor_minute: s.restart_anchor_minute,
                restart_interval_choices: RESTART_INTERVALS_HOURS.to_vec(),
            }
        }
        None => ScheduleRangesDto {
            enabled: false,
            mode: ScheduleMode::Ranges.as_str().to_string(),
            timezone: "Europe/Paris".into(),
            ranges: Vec::new(),
            warn_minutes: 10,
            next_opening: None,
            disabled_restart_keys: Vec::new(),
            restart_interval_hours: None,
            restart_anchor_minute: 0,
            next_restart: None,
            restart_interval_choices: RESTART_INTERVALS_HOURS.to_vec(),
        },
    };
    Ok(Json(dto))
}

/// PUT /api/games/servers/{server_id}/schedule-ranges
pub async fn save_schedule_ranges(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    headers: HeaderMap,
    Query(q): Query<ActorQuery>,
    Json(dto): Json<SaveScheduleRangesDto>,
) -> Result<Json<ScheduleRangesDto>, ApiError> {
    use platform_core::nexus::domain::entities::game::schedule::{
        ScheduleMode, RESTART_INTERVALS_HOURS, TOUS_LES_JOURS,
    };
    use platform_core::nexus::domain::errors::DomainError;
    use platform_core::nexus::ports::outbound::game::schedule_repository::ScheduleSettings;

    let actor = acteur(&state, &headers, server_id, q.actor_id.as_deref()).await?;

    // Un fuseau inconnu ferait tourner le serveur a contretemps sans que
    // personne ne comprenne : on refuse a la saisie plutot que de s'en
    // apercevoir un soir d'ouverture.
    if dto.timezone.parse::<chrono_tz::Tz>().is_err() {
        return Err(DomainError::ValidationError(format!(
            "fuseau horaire inconnu : {}",
            dto.timezone
        ))
        .into());
    }

    let mode = dto
        .mode
        .as_deref()
        .map(ScheduleMode::from_str)
        .unwrap_or(ScheduleMode::Ranges);

    if dto.restart_anchor_minute > 59 {
        return Err(DomainError::ValidationError(
            "la minute de redemarrage doit etre comprise entre 0 et 59".into(),
        )
        .into());
    }

    if let Some(intervalle) = dto.restart_interval_hours {
        // Seuls les diviseurs de 24 gardent les creneaux a la meme heure d'un
        // jour a l'autre. Refuser ici evite d'enregistrer une cadence qui
        // rendrait fausse l'annonce du lendemain.
        if !RESTART_INTERVALS_HOURS.contains(&intervalle) {
            return Err(DomainError::ValidationError(format!(
                "cadence de redemarrage non proposee : {intervalle} h"
            ))
            .into());
        }
    }

    if mode == ScheduleMode::Restart && dto.enabled && dto.restart_interval_hours.is_none() {
        return Err(DomainError::ValidationError(
            "choisis une cadence de redemarrage avant d'activer la permanence".into(),
        )
        .into());
    }

    for plage in &dto.ranges {
        // Bits au-dela de dimanche : la charge utile ne vient pas de notre
        // formulaire. On refuse plutot que de les ignorer en silence, sans
        // quoi une plage « lundi » pourrait en realite en cacher d'autres.
        if plage.days > TOUS_LES_JOURS {
            return Err(
                DomainError::ValidationError("jours de la semaine invalides".into()).into(),
            );
        }
        if plage.start_minute >= 1440 || plage.end_minute >= 1440 {
            return Err(DomainError::ValidationError(
                "une heure doit tenir dans la journee".into(),
            )
            .into());
        }
        // Debut egal a la fin : la plage dure zero minute, ou vingt-quatre
        // heures selon la lecture. Trop ambigu pour etre accepte.
        if plage.start_minute == plage.end_minute {
            return Err(DomainError::ValidationError(
                "une plage doit avoir une duree : ajuste l'heure de fin".into(),
            )
            .into());
        }
    }

    // L'exigence ne vaut que pour les plages : une permanence n'en a aucune,
    // et lui en reclamer une empecherait purement et simplement de l'activer.
    if mode == ScheduleMode::Ranges && dto.enabled && dto.ranges.is_empty() {
        return Err(DomainError::ValidationError(
            "ajoute au moins une plage avant d'activer les horaires".into(),
        )
        .into());
    }
    // Toutes les plages sans aucun jour coche : le serveur n'ouvrirait jamais,
    // alors que l'interface annonce des horaires actifs. Le refus est plus
    // honnete qu'un serveur muet dont personne ne comprend le silence.
    if mode == ScheduleMode::Ranges
        && dto.enabled
        && !dto.ranges.is_empty()
        && dto.ranges.iter().all(|plage| plage.days == 0)
    {
        return Err(DomainError::ValidationError(
            "coche au moins un jour de la semaine pour tes plages".into(),
        )
        .into());
    }

    state
        .game_schedule_repo
        .upsert(
            server_id,
            &ScheduleSettings {
                enabled: dto.enabled,
                mode,
                timezone: dto.timezone.clone(),
                ranges: dto.ranges.clone(),
                warn_minutes: dto.warn_minutes.min(120),
                restart_interval_hours: dto.restart_interval_hours,
                restart_anchor_minute: dto.restart_anchor_minute,
            },
            Some(&actor),
        )
        .await?;

    // Les horaires prennent la main sur le redemarrage programme du jeu : un
    // serveur qui ferme et rouvre chaque jour redemarre deja, et le cron
    // risquerait de rallumer un conteneur qu'on vient d'eteindre.
    let mut neutralisees = Vec::new();
    if dto.enabled {
        if let Ok(detail) = state.game_servers_uc.get(server_id).await {
            let mut config = detail.config.clone();
            for cle in CLES_REDEMARRAGE_AUTO {
                if let Some(valeur) = config.get_mut(*cle) {
                    let actif =
                        !matches!(valeur.trim().to_ascii_lowercase().as_str(), "" | "false");
                    if actif {
                        *valeur = if cle.contains("CRON") {
                            String::new()
                        } else {
                            "false".to_string()
                        };
                        neutralisees.push((*cle).to_string());
                    }
                }
            }
            if !neutralisees.is_empty() {
                state
                    .game_servers_uc
                    .update_config(server_id, config, &actor)
                    .await?;
            }
        }
    }

    let mut resultat = get_schedule_ranges(State(state), Path(server_id)).await?;
    resultat.0.disabled_restart_keys = neutralisees;
    Ok(resultat)
}
