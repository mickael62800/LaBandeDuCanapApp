//! Bootstrap : câblage des services `platform_core::nexus` avec les adaptateurs Postgres.

use std::sync::Arc;

use platform_core::nexus::application::coussin_bet_service::CoussinBetService;
use platform_core::nexus::application::coussin_insurance_service::CoussinInsuranceService;
use platform_core::nexus::application::coussin_inventory_service::CoussinInventoryService;
use platform_core::nexus::application::coussin_prime_service::CoussinPrimeService;
use platform_core::nexus::application::coussin_service::CoussinService;
use platform_core::nexus::application::coussin_steal_service::CoussinStealService;
use platform_core::nexus::application::game::manage_game_servers_service::ManageGameServersService;
use platform_core::nexus::application::game::manage_templates_service::ManageGameTemplatesService;
use platform_core::nexus::application::grand_salon_service::GrandSalonService;
use platform_core::nexus::application::play_wheel_service::PlayWheelService;
use platform_core::nexus::application::wallet_service::WalletService;
use platform_core::nexus::ports::inbound::coussin_bet::CoussinBetUseCase;
use platform_core::nexus::ports::inbound::coussin_insurance::CoussinInsuranceUseCase;
use platform_core::nexus::ports::inbound::coussin_inventory::CoussinInventoryUseCase;
use platform_core::nexus::ports::inbound::coussin_prime::CoussinPrimeUseCase;
use platform_core::nexus::ports::inbound::coussin_profile::CoussinCombatUseCase;
use platform_core::nexus::ports::inbound::coussin_profile::CoussinProfileUseCase;
use platform_core::nexus::ports::inbound::coussin_steal::CoussinStealUseCase;
use platform_core::nexus::ports::inbound::game::manage_game_servers::ManageGameServersUseCase;
use platform_core::nexus::ports::inbound::game::manage_game_templates::ManageGameTemplatesUseCase;
use platform_core::nexus::ports::inbound::get_wallet::GetWalletUseCase;
use platform_core::nexus::ports::inbound::play_wheel::PlayWheelUseCase;
use platform_core::nexus::ports::inbound::transfer_coins::TransferCoinsUseCase;
use platform_core::nexus::ports::inbound::wallet_history::GetWalletHistoryUseCase;
use platform_core::nexus::ports::inbound::wallet_leaderboard::GetWalletLeaderboardUseCase;
use platform_core::nexus::ports::outbound::casino::game_repository::GameRepository;
use platform_core::nexus::ports::outbound::casino::game_sync_repository::GameSyncRepository;
use platform_core::nexus::ports::outbound::coussin_bet_repository::CoussinBetRepository;
use platform_core::nexus::ports::outbound::coussin_insurance_repository::CoussinInsuranceRepository;
use platform_core::nexus::ports::outbound::coussin_inventory_repository::CoussinInventoryRepository;
use platform_core::nexus::ports::outbound::coussin_prime_repository::CoussinPrimeRepository;
use platform_core::nexus::ports::outbound::coussin_repository::CoussinRepository;
use platform_core::nexus::ports::outbound::coussin_steal_repository::CoussinStealRepository;
use platform_core::nexus::ports::outbound::events::EventPublisher;
use platform_core::nexus::ports::outbound::game::backup_repository::GameBackupRepository;
use platform_core::nexus::ports::outbound::game::container_runtime::ContainerRuntime;
use platform_core::nexus::ports::outbound::game::game_audit_repository::GameAuditRepository;
use platform_core::nexus::ports::outbound::game::game_server_repository::GameServerRepository;
use platform_core::nexus::ports::outbound::game::game_session_repository::{
    GameSessionRegistrationRepository, GameTemplateSettingsRepository,
};
use platform_core::nexus::ports::outbound::game::game_template_repository::GameTemplateRepository;
use platform_core::nexus::ports::outbound::game::player_session_repository::PlayerSessionRepository;
use platform_core::nexus::ports::outbound::game::port_allocator::PortAllocator;
use platform_core::nexus::ports::outbound::game::rcon_client::RconClient;
use platform_core::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository;
use sqlx::postgres::PgPoolOptions;

use crate::nexus::adapters::outbound::events::noop_publisher::NoopEventPublisher;
use crate::nexus::adapters::outbound::events::redis_publisher::RedisEventPublisher;
use crate::nexus::adapters::outbound::game_runtime::http_runtime::HttpGameRuntime;
use crate::nexus::adapters::outbound::game_runtime::noop_runtime::NoopContainerRuntime;
use crate::nexus::adapters::outbound::game_runtime::rcon_pooled::PooledRconClient;
use crate::nexus::adapters::outbound::game_runtime::redis_port_allocator::RedisPortAllocator;
use crate::nexus::adapters::outbound::postgres::casino::game_repository::PgGameRepository;
use crate::nexus::adapters::outbound::postgres::casino::game_sync_repository::PgGameSyncRepository;
use crate::nexus::adapters::outbound::postgres::coussin_bet_repository::PgCoussinBetRepository;
use crate::nexus::adapters::outbound::postgres::coussin_insurance_repository::PgCoussinInsuranceRepository;
use crate::nexus::adapters::outbound::postgres::coussin_inventory_repository::PgCoussinInventoryRepository;
use crate::nexus::adapters::outbound::postgres::coussin_prime_repository::PgCoussinPrimeRepository;
use crate::nexus::adapters::outbound::postgres::coussin_repository::PgCoussinRepository;
use crate::nexus::adapters::outbound::postgres::coussin_steal_repository::PgCoussinStealRepository;
use crate::nexus::adapters::outbound::postgres::game::audit_repository::PgGameAuditRepository;
use crate::nexus::adapters::outbound::postgres::game::backup_repository::PgGameBackupRepository;
use crate::nexus::adapters::outbound::postgres::game::config_repository::PgGameServerConfigRepository;
use crate::nexus::adapters::outbound::postgres::game::player_session_repository::PgPlayerSessionRepository;
use crate::nexus::adapters::outbound::postgres::game::server_repository::PgGameServerRepository;
use crate::nexus::adapters::outbound::postgres::game::session_repository::{
    PgGameSessionRegistrationRepository, PgGameTemplateSettingsRepository,
};
use crate::nexus::adapters::outbound::postgres::game::template_repository::PgGameTemplateRepository;
use crate::nexus::adapters::outbound::postgres::grand_salon_repository::PgGrandSalonRepository;
use crate::nexus::adapters::outbound::postgres::system::bot_config_repository::PgBotConfigRepository;
use crate::nexus::adapters::outbound::postgres::wallet_repository::PgWalletRepository;
use crate::nexus::adapters::outbound::postgres::wheel_repository::PgWheelRepository;

#[derive(Clone)]
pub struct AppState {
    pub job_pool: sqlx::PgPool,
    pub grand_salon: Arc<GrandSalonService>,
    pub play_wheel: Arc<dyn PlayWheelUseCase>,
    pub wheel_cases: Arc<dyn platform_core::nexus::ports::inbound::wheel_cases::ManageWheelCasesUseCase>,
    pub get_wallet: Arc<dyn GetWalletUseCase>,
    pub transfer_coins: Arc<dyn TransferCoinsUseCase>,
    pub wallet_history: Arc<dyn GetWalletHistoryUseCase>,
    pub wallet_leaderboard: Arc<dyn GetWalletLeaderboardUseCase>,
    pub coussin_profile: Arc<dyn CoussinProfileUseCase>,
    /// Acces direct au depot des bagarres, pour le job qui ferme les defis
    /// restes sans reponse. Les cas d'usage joueur passent par les services.
    pub coussin_repo: Arc<dyn CoussinRepository>,
    /// Reglages d'alerte des serveurs de jeu. L'URL de webhook qu'ils portent
    /// est un secret : elle ne quitte jamais cette couche vers le navigateur.
    pub game_alert_repo: Arc<
        dyn platform_core::nexus::ports::outbound::game::alert_repository::GameAlertRepository,
    >,
    /// Plages d'ouverture recurrentes des serveurs de jeu.
    pub game_schedule_repo: Arc<
        dyn platform_core::nexus::ports::outbound::game::schedule_repository::GameScheduleRepository,
    >,
    pub coussin_combat: Arc<dyn CoussinCombatUseCase>,
    pub coussin_inventory: Arc<dyn CoussinInventoryUseCase>,
    pub coussin_insurance: Arc<dyn CoussinInsuranceUseCase>,
    pub coussin_steal: Arc<dyn CoussinStealUseCase>,
    pub coussin_prime: Arc<dyn CoussinPrimeUseCase>,
    pub coussin_bet: Arc<dyn CoussinBetUseCase>,
    // ── Hauts faits ──
    pub achievements_uc: Arc<
        dyn platform_core::nexus::ports::inbound::achievements::ManageAchievementsUseCase,
    >,
    // ── Game Portal ──
    pub game_servers_uc: Arc<dyn ManageGameServersUseCase>,
    pub game_templates_uc: Arc<dyn ManageGameTemplatesUseCase>,
    /// Adapters exposes pour les endpoints internes /jobs/* (worker) et
    /// quelques handlers qui accedent directement aux repos.
    pub game_server_repo: Arc<dyn GameServerRepository>,
    pub game_template_repo: Arc<dyn GameTemplateRepository>,
    pub game_template_settings_repo: Arc<dyn GameTemplateSettingsRepository>,
    pub game_session_reg_repo: Arc<dyn GameSessionRegistrationRepository>,
    pub game_audit_repo: Arc<dyn GameAuditRepository>,
    pub game_session_repo: Arc<dyn PlayerSessionRepository>,
    pub game_container_runtime: Arc<dyn ContainerRuntime>,
    /// Archives des mondes, alimentees par le redemarrage programme.
    pub game_backup_repo: Arc<dyn GameBackupRepository>,
    pub game_rcon_client: Arc<dyn RconClient>,
    pub game_port_allocator: Arc<dyn PortAllocator>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    /// Catalogue des jeux mentionnables (games/panels).
    pub game_repo: Arc<dyn GameRepository>,
    /// Derniere photographie Discord de chaque guilde, deposee par le bot.
    /// Sert a constater les divergences ; ne repare rien par elle-meme.
    pub game_sync_repo: Arc<dyn GameSyncRepository>,
    /// Publie les evenements consommes par le bot (salons de session).
    pub events: Arc<dyn EventPublisher>,
    pub discord_api:
        Arc<dyn platform_core::nexus::ports::outbound::system::discord_api_repository::DiscordApiRepository>,
    /// Toutes les routes `/api` exigent `Authorization: Bearer <key>`.
    ///
    /// Non optionnel : le bootstrap refuse de demarrer sans cle (cf. la lecture
    /// de `NEXUS_API_KEY`). Le type porte donc la garantie — il n'existe pas
    /// d'etat « API servie sans authentification » a representer.
    pub api_key: String,
    /// Si Some et non vide, `/metrics` exige `Authorization: Bearer <token>`.
    ///
    /// Vide = ouvert, ce qui convient tant que le port n'est joignable que
    /// depuis le reseau Docker interne ou vit Prometheus.
    pub metrics_token: Option<String>,
    /// Serveur Discord unique servi par cette installation.
    ///
    /// `None` = verrou desactive. Voir `single_guild` cote HTTP : Nexus
    /// expose sa propre surface, il lui faut donc son propre verrou —
    /// celui de sentinel-api ne le protege pas.
    pub guild_id: Option<String>,
}

/// Connecte le pool Postgres (NEXUS_DATABASE_URL), applique les migrations
/// `nexus-api/migrations/`, et construit l'AppState.
///
/// Env game-portal :
///   - NEXUS_GAME_RUNTIME = docker | noop (defaut : noop). En mode docker,
///     fallback automatique sur noop si le socket Docker est indisponible.
///   - REDIS_URL (defaut redis://127.0.0.1:6379) : allocation atomique des
///     ports via SETNX.
pub async fn build_state() -> Result<AppState, Box<dyn std::error::Error>> {
    let db_url = std::env::var("NEXUS_DATABASE_URL")
        .map_err(|_| "NEXUS_DATABASE_URL manquante (ex: postgres://user:pass@host/nexus)")?;

    // Pool partage par les requetes ET les verrous de jobs (`job_pool`). Chaque
    // job planifie (health-check, auto-start, reveal-ip...) tient une connexion
    // pour son verrou d'avance pendant TOUTE sa duree : a 5, quelques jobs qui
    // se chevauchent suffisaient a epuiser le pool et a faire echouer les
    // acquisitions en `pool timed out`. Defaut releve a 20, configurable ; reste
    // bien sous le pool serveur de nexus-pgbouncer et le max_connections Postgres.
    let max_conns = std::env::var("NEXUS_DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(20);
    let pool = PgPoolOptions::new()
        .max_connections(max_conns)
        .connect(&db_url)
        .await?;

    sqlx::migrate!("./migrations/nexus").run(&pool).await?;

    // Declare en tete : plusieurs services le lisent pour connaitre
    // l'equilibre du jeu (taux de vol, delais, bornes de transfert).
    let bot_config_repo: Arc<dyn BotConfigRepository> =
        Arc::new(PgBotConfigRepository::new(pool.clone()));

    let wheel_repo = Arc::new(PgWheelRepository::new(pool.clone()));
    let wallet_repo = Arc::new(PgWalletRepository::new(pool.clone()));
    let grand_salon = Arc::new(GrandSalonService::new(
        Arc::new(PgGrandSalonRepository::new(pool.clone())),
        1_000,
    ));
    let wheel_cases: Arc<
        dyn platform_core::nexus::ports::inbound::wheel_cases::ManageWheelCasesUseCase,
    > = Arc::new(
        platform_core::nexus::application::wheel_cases_service::WheelCasesService::new(
            wheel_repo.clone(),
        ),
    );
    let service = Arc::new(PlayWheelService::new(
        wheel_repo,
        wallet_repo.clone(),
        bot_config_repo.clone(),
    ));
    let wallet_service = Arc::new(WalletService::new(wallet_repo, bot_config_repo.clone()));
    let coussin_cooldowns: Arc<dyn platform_core::nexus::ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository> =
        Arc::new(crate::nexus::adapters::outbound::postgres::coussin_cooldown_repository::PgCoussinCooldownRepository::new(pool.clone()));
    let coussin_repo: Arc<dyn CoussinRepository> = Arc::new(PgCoussinRepository::new(pool.clone()));
    let game_schedule_repo: Arc<
        dyn platform_core::nexus::ports::outbound::game::schedule_repository::GameScheduleRepository,
    > = Arc::new(
        crate::nexus::adapters::outbound::postgres::game::schedule_repository::PgGameScheduleRepository::new(
            pool.clone(),
        ),
    );
    let game_alert_repo: Arc<
        dyn platform_core::nexus::ports::outbound::game::alert_repository::GameAlertRepository,
    > = Arc::new(
        crate::nexus::adapters::outbound::postgres::game::alert_repository::PgGameAlertRepository::new(
            pool.clone(),
        ),
    );
    let coussin_profile: Arc<dyn CoussinProfileUseCase> = Arc::new(CoussinService::new(
        coussin_repo.clone(),
        bot_config_repo.clone(),
        coussin_cooldowns.clone(),
    ));
    let coussin_combat: Arc<dyn CoussinCombatUseCase> = Arc::new(CoussinService::new(
        Arc::new(PgCoussinRepository::new(pool.clone())),
        bot_config_repo.clone(),
        coussin_cooldowns.clone(),
    ));
    let coussin_inventory_repo: Arc<dyn CoussinInventoryRepository> =
        Arc::new(PgCoussinInventoryRepository::new(pool.clone()));
    let coussin_inventory: Arc<dyn CoussinInventoryUseCase> = Arc::new(
        CoussinInventoryService::new(coussin_inventory_repo, bot_config_repo.clone()),
    );
    let coussin_insurance_repo: Arc<dyn CoussinInsuranceRepository> =
        Arc::new(PgCoussinInsuranceRepository::new(pool.clone()));
    let coussin_insurance: Arc<dyn CoussinInsuranceUseCase> = Arc::new(
        CoussinInsuranceService::new(coussin_insurance_repo, bot_config_repo.clone()),
    );
    let coussin_steal_repo: Arc<dyn CoussinStealRepository> =
        Arc::new(PgCoussinStealRepository::new(pool.clone()));
    let coussin_steal: Arc<dyn CoussinStealUseCase> = Arc::new(CoussinStealService::new(
        coussin_steal_repo,
        // La defense de la victime pese sur le jet : la fouille a besoin des
        // profils, pas seulement des porte-monnaie.
        coussin_repo.clone(),
        bot_config_repo.clone(),
    ));
    let coussin_prime_repo: Arc<dyn CoussinPrimeRepository> =
        Arc::new(PgCoussinPrimeRepository::new(pool.clone()));
    let coussin_prime: Arc<dyn CoussinPrimeUseCase> = Arc::new(CoussinPrimeService::new(
        coussin_prime_repo,
        bot_config_repo.clone(),
        coussin_cooldowns.clone(),
    ));
    let coussin_bet_repo: Arc<dyn CoussinBetRepository> =
        Arc::new(PgCoussinBetRepository::new(pool.clone()));
    let coussin_bet: Arc<dyn CoussinBetUseCase> = Arc::new(CoussinBetService::new(
        coussin_bet_repo,
        bot_config_repo.clone(),
        coussin_cooldowns.clone(),
    ));

    // ── Game Portal : repos Postgres ──
    let game_server_repo: Arc<dyn GameServerRepository> =
        Arc::new(PgGameServerRepository::new(pool.clone()));
    let game_template_repo: Arc<dyn GameTemplateRepository> =
        Arc::new(PgGameTemplateRepository::new(pool.clone()));
    let game_config_repo = Arc::new(PgGameServerConfigRepository::new(pool.clone()));
    let game_audit_repo: Arc<dyn GameAuditRepository> =
        Arc::new(PgGameAuditRepository::new(pool.clone()));
    let game_session_repo: Arc<dyn PlayerSessionRepository> =
        Arc::new(PgPlayerSessionRepository::new(pool.clone()));
    let game_template_settings_repo: Arc<dyn GameTemplateSettingsRepository> =
        Arc::new(PgGameTemplateSettingsRepository::new(pool.clone()));
    let game_session_reg_repo: Arc<dyn GameSessionRegistrationRepository> =
        Arc::new(PgGameSessionRegistrationRepository::new(pool.clone()));
    let game_repo: Arc<dyn GameRepository> = Arc::new(PgGameRepository::new(pool.clone()));
    let game_sync_repo: Arc<dyn GameSyncRepository> =
        Arc::new(PgGameSyncRepository::new(pool.clone()));

    // ── Game Portal : runtime container (docker | noop) ──
    // NEXUS_GAME_RUNTIME=docker passe par `docker-agent` ; toute autre valeur
    // (ou l'absence de la variable, ou un agent injoignable) => operations de
    // cycle de vie refusees, mais le listing et la config continuent de
    // fonctionner.
    //
    // « docker » designe desormais l'agent, plus le socket local : ce processus
    // ne monte plus `/var/run/docker.sock`. La valeur de la variable est
    // conservee telle quelle pour ne pas casser les .env existants.
    let runtime_mode = std::env::var("NEXUS_GAME_RUNTIME").unwrap_or_else(|_| "noop".into());
    let container_runtime: Arc<dyn ContainerRuntime> = if runtime_mode == "docker" {
        let agent_url =
            std::env::var("DOCKER_AGENT_URL").unwrap_or_else(|_| "http://docker-agent:8095".into());
        // Jeton de la surface `/game/*` UNIQUEMENT. Ce processus ne doit pas
        // porter `DOCKER_AGENT_TOKEN`, qui ouvre l'administration de l'hote
        // (arret et purge de n'importe quel conteneur).
        let agent_token = std::env::var("DOCKER_AGENT_GAME_TOKEN").unwrap_or_default();
        if agent_token.trim().is_empty() {
            tracing::warn!(
                "DOCKER_AGENT_GAME_TOKEN absent — Game Portal lifecycle inactif (l'agent refusera tout)"
            );
        }
        Arc::new(HttpGameRuntime::connect(agent_url, agent_token).await)
    } else if runtime_mode == "mock" {
        tracing::info!("NEXUS_GAME_RUNTIME=mock — runtime container simule en memoire");
        Arc::new(platform_core::nexus::ports::outbound::game::container_runtime::MockContainerRuntime::new())
    } else {
        tracing::info!("NEXUS_GAME_RUNTIME={runtime_mode} — runtime container noop");
        Arc::new(NoopContainerRuntime)
    };

    let rcon_client: Arc<dyn RconClient> = Arc::new(PooledRconClient::default());

    // Le client redis ne se connecte pas a l'open (lazy) : une URL par defaut
    // ne coute rien tant que l'allocation de port n'est pas sollicitee.
    let redis_url = std::env::var("NEXUS_REDIS_URL")
        .or_else(|_| std::env::var("REDIS_URL"))
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let redis_client = redis::Client::open(redis_url.as_str())
        .map_err(|e| format!("REDIS_URL invalide ({redis_url}): {e}"))?;
    let port_allocator: Arc<dyn PortAllocator> = Arc::new(RedisPortAllocator::new(redis_client));

    let events: Arc<dyn EventPublisher> = std::env::var("NEXUS_REDIS_URL")
        .or_else(|_| std::env::var("REDIS_URL"))
        .map_or_else(
            |_| {
                tracing::warn!(
                    "NEXUS_REDIS_URL absente — events desactives : le bot ne creera pas \
                 les salons de session game-portal"
                );
                Arc::new(NoopEventPublisher) as Arc<dyn EventPublisher>
            },
            |url| match RedisEventPublisher::new(&url) {
                Ok(p) => Arc::new(p),
                Err(e) => {
                    tracing::warn!(error = %e, "NEXUS_REDIS_URL invalide — events desactives");
                    Arc::new(NoopEventPublisher)
                }
            },
        );

    let discord_token = std::env::var("NEXUS_DISCORD_TOKEN").unwrap_or_default();
    let discord_api: Arc<
        dyn platform_core::nexus::ports::outbound::system::discord_api_repository::DiscordApiRepository,
    > = Arc::new(
        crate::nexus::adapters::outbound::system::discord_api::ReqwestDiscordApiClient::new(discord_token),
    );

    // ── Hauts faits ──
    let achievements_uc: Arc<
        dyn platform_core::nexus::ports::inbound::achievements::ManageAchievementsUseCase,
    > = Arc::new(
        platform_core::nexus::application::achievements_service::AchievementsService::new(Arc::new(
            crate::nexus::adapters::outbound::postgres::achievement_repository::PgAchievementRepository::new(
                pool.clone(),
            ),
        )),
    );

    // ── Game Portal : use cases ──
    let game_servers_uc: Arc<dyn ManageGameServersUseCase> = Arc::new(ManageGameServersService {
        server_repo: game_server_repo.clone(),
        template_repo: game_template_repo.clone(),
        config_repo: game_config_repo,
        audit_repo: game_audit_repo.clone(),
        container_runtime: container_runtime.clone(),
        rcon_client: rcon_client.clone(),
        port_allocator: port_allocator.clone(),
        bot_config: bot_config_repo.clone(),
    });
    let game_templates_uc: Arc<dyn ManageGameTemplatesUseCase> = Arc::new(
        ManageGameTemplatesService::new(game_template_repo.clone(), bot_config_repo.clone()),
    );

    // Aligne sur `ops-api` : on refuse de demarrer plutot que de servir ouvert.
    //
    // Avant, une cle absente ou vide donnait `None`, et `require_optional`
    // laissait alors passer TOUTES les routes `/api` — cycle de vie des
    // conteneurs compris. Nexus est la seule API capable de lancer des
    // conteneurs sur l'hote, et c'etait la seule des quatre a echouer en
    // s'OUVRANT. Le compose aggravait le cas (`${NEXUS_API_KEY:-}`, defaut
    // vide), et un `warn!` dans les logs ne protege rien.
    //
    // Le seuil de 16 caracteres est celui de `sentinel-api` : une cle courte
    // est devinable, et la refuser au demarrage evite de decouvrir le probleme
    // le jour ou quelqu'un la teste.
    let api_key = std::env::var("NEXUS_API_KEY")
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| k.len() >= 16)
        .unwrap_or_else(|| {
            tracing::error!(
                "NEXUS_API_KEY manquante, vide ou trop courte (16 caracteres minimum) — \
                 arret : servir cette API sans authentification ouvrirait le cycle de vie \
                 des conteneurs de l'hote"
            );
            std::process::exit(1)
        });
    let metrics_token = std::env::var("NEXUS_METRICS_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());
    // Meme variable que sentinel-api et que le conteneur web : une seule
    // source de verite pour « de quel serveur parle cette installation ».
    let guild_id = std::env::var("PUBLIC_GUILD_ID")
        .or_else(|_| std::env::var("GUILD_ID"))
        .ok()
        .map(|g| g.trim().to_string())
        .filter(|g| !g.is_empty());
    match &guild_id {
        Some(g) => tracing::info!(guild_id = %g, "mono-serveur : verrou actif"),
        None => tracing::warn!("PUBLIC_GUILD_ID absente — toutes les guildes sont acceptees"),
    }
    Ok(AppState {
        job_pool: pool.clone(),
        grand_salon,
        play_wheel: service,
        wheel_cases,
        get_wallet: wallet_service.clone(),
        transfer_coins: wallet_service.clone(),
        wallet_history: wallet_service.clone(),
        wallet_leaderboard: wallet_service,
        coussin_profile,
        coussin_repo,
        game_alert_repo,
        game_schedule_repo,
        coussin_combat,
        coussin_inventory,
        coussin_insurance,
        coussin_steal,
        coussin_prime,
        coussin_bet,
        achievements_uc,
        game_servers_uc,
        game_templates_uc,
        game_server_repo,
        game_template_repo,
        game_template_settings_repo,
        game_session_reg_repo,
        game_audit_repo,
        game_session_repo,
        game_container_runtime: container_runtime,
        game_backup_repo: Arc::new(PgGameBackupRepository::new(pool.clone())),
        game_rcon_client: rcon_client,
        game_port_allocator: port_allocator,
        bot_config_repo,
        game_repo,
        game_sync_repo,
        events,
        discord_api,
        api_key,
        metrics_token,
        guild_id,
    })
}
