//! Bootstrap : cablage des services nexus-core avec les adapters Postgres.

use std::sync::Arc;

use nexus_core::application::coussin_bet_service::CoussinBetService;
use nexus_core::application::coussin_insurance_service::CoussinInsuranceService;
use nexus_core::application::coussin_inventory_service::CoussinInventoryService;
use nexus_core::application::coussin_prime_service::CoussinPrimeService;
use nexus_core::application::coussin_service::CoussinService;
use nexus_core::application::coussin_steal_service::CoussinStealService;
use nexus_core::application::game::manage_game_servers_service::ManageGameServersService;
use nexus_core::application::game::manage_templates_service::ManageGameTemplatesService;
use nexus_core::application::grand_salon_service::GrandSalonService;
use nexus_core::application::play_wheel_service::PlayWheelService;
use nexus_core::application::wallet_service::WalletService;
use nexus_core::ports::inbound::coussin_bet::CoussinBetUseCase;
use nexus_core::ports::inbound::coussin_insurance::CoussinInsuranceUseCase;
use nexus_core::ports::inbound::coussin_inventory::CoussinInventoryUseCase;
use nexus_core::ports::inbound::coussin_prime::CoussinPrimeUseCase;
use nexus_core::ports::inbound::coussin_profile::CoussinCombatUseCase;
use nexus_core::ports::inbound::coussin_profile::CoussinProfileUseCase;
use nexus_core::ports::inbound::coussin_steal::CoussinStealUseCase;
use nexus_core::ports::inbound::game::manage_game_servers::ManageGameServersUseCase;
use nexus_core::ports::inbound::game::manage_game_templates::ManageGameTemplatesUseCase;
use nexus_core::ports::inbound::get_wallet::GetWalletUseCase;
use nexus_core::ports::inbound::play_wheel::PlayWheelUseCase;
use nexus_core::ports::inbound::transfer_coins::TransferCoinsUseCase;
use nexus_core::ports::inbound::wallet_history::GetWalletHistoryUseCase;
use nexus_core::ports::inbound::wallet_leaderboard::GetWalletLeaderboardUseCase;
use nexus_core::ports::outbound::casino::game_repository::GameRepository;
use nexus_core::ports::outbound::coussin_bet_repository::CoussinBetRepository;
use nexus_core::ports::outbound::coussin_insurance_repository::CoussinInsuranceRepository;
use nexus_core::ports::outbound::coussin_inventory_repository::CoussinInventoryRepository;
use nexus_core::ports::outbound::coussin_prime_repository::CoussinPrimeRepository;
use nexus_core::ports::outbound::coussin_repository::CoussinRepository;
use nexus_core::ports::outbound::coussin_steal_repository::CoussinStealRepository;
use nexus_core::ports::outbound::events::EventPublisher;
use nexus_core::ports::outbound::game::container_runtime::ContainerRuntime;
use nexus_core::ports::outbound::game::game_audit_repository::GameAuditRepository;
use nexus_core::ports::outbound::game::game_server_repository::GameServerRepository;
use nexus_core::ports::outbound::game::game_session_repository::{
    GameSessionRegistrationRepository, GameTemplateSettingsRepository,
};
use nexus_core::ports::outbound::game::game_template_repository::GameTemplateRepository;
use nexus_core::ports::outbound::game::player_session_repository::PlayerSessionRepository;
use nexus_core::ports::outbound::game::port_allocator::PortAllocator;
use nexus_core::ports::outbound::game::rcon_client::RconClient;
use nexus_core::ports::outbound::system::bot_config_repository::BotConfigRepository;
use sqlx::postgres::PgPoolOptions;

use crate::adapters::outbound::events::noop_publisher::NoopEventPublisher;
use crate::adapters::outbound::events::redis_publisher::RedisEventPublisher;
use crate::adapters::outbound::game_runtime::docker_runtime::{
    make_docker_client, DockerContainerRuntime,
};
use crate::adapters::outbound::game_runtime::noop_runtime::NoopContainerRuntime;
use crate::adapters::outbound::game_runtime::rcon_pooled::PooledRconClient;
use crate::adapters::outbound::game_runtime::redis_port_allocator::RedisPortAllocator;
use crate::adapters::outbound::postgres::casino::game_repository::PgGameRepository;
use crate::adapters::outbound::postgres::coussin_bet_repository::PgCoussinBetRepository;
use crate::adapters::outbound::postgres::coussin_insurance_repository::PgCoussinInsuranceRepository;
use crate::adapters::outbound::postgres::coussin_inventory_repository::PgCoussinInventoryRepository;
use crate::adapters::outbound::postgres::coussin_prime_repository::PgCoussinPrimeRepository;
use crate::adapters::outbound::postgres::coussin_repository::PgCoussinRepository;
use crate::adapters::outbound::postgres::coussin_steal_repository::PgCoussinStealRepository;
use crate::adapters::outbound::postgres::game::audit_repository::PgGameAuditRepository;
use crate::adapters::outbound::postgres::game::config_repository::PgGameServerConfigRepository;
use crate::adapters::outbound::postgres::game::player_session_repository::PgPlayerSessionRepository;
use crate::adapters::outbound::postgres::game::server_repository::PgGameServerRepository;
use crate::adapters::outbound::postgres::game::session_repository::{
    PgGameSessionRegistrationRepository, PgGameTemplateSettingsRepository,
};
use crate::adapters::outbound::postgres::game::template_repository::PgGameTemplateRepository;
use crate::adapters::outbound::postgres::grand_salon_repository::PgGrandSalonRepository;
use crate::adapters::outbound::postgres::system::bot_config_repository::PgBotConfigRepository;
use crate::adapters::outbound::postgres::wallet_repository::PgWalletRepository;
use crate::adapters::outbound::postgres::wheel_repository::PgWheelRepository;

#[derive(Clone)]
pub struct AppState {
    pub grand_salon: Arc<GrandSalonService>,
    pub play_wheel: Arc<dyn PlayWheelUseCase>,
    pub wheel_cases: Arc<dyn nexus_core::ports::inbound::wheel_cases::ManageWheelCasesUseCase>,
    pub get_wallet: Arc<dyn GetWalletUseCase>,
    pub transfer_coins: Arc<dyn TransferCoinsUseCase>,
    pub wallet_history: Arc<dyn GetWalletHistoryUseCase>,
    pub wallet_leaderboard: Arc<dyn GetWalletLeaderboardUseCase>,
    pub coussin_profile: Arc<dyn CoussinProfileUseCase>,
    pub coussin_combat: Arc<dyn CoussinCombatUseCase>,
    pub coussin_inventory: Arc<dyn CoussinInventoryUseCase>,
    pub coussin_insurance: Arc<dyn CoussinInsuranceUseCase>,
    pub coussin_steal: Arc<dyn CoussinStealUseCase>,
    pub coussin_prime: Arc<dyn CoussinPrimeUseCase>,
    pub coussin_bet: Arc<dyn CoussinBetUseCase>,
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
    pub game_rcon_client: Arc<dyn RconClient>,
    pub game_port_allocator: Arc<dyn PortAllocator>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    /// Catalogue des jeux mentionnables (games/panels).
    pub game_repo: Arc<dyn GameRepository>,
    /// Publie les evenements consommes par le bot (salons de session).
    pub events: Arc<dyn EventPublisher>,
    pub discord_api:
        Arc<dyn nexus_core::ports::outbound::system::discord_api_repository::DiscordApiRepository>,
    /// Si Some, toutes les routes /api exigent `Authorization: Bearer <key>`.
    pub api_key: Option<String>,
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

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

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
    let wheel_cases: Arc<dyn nexus_core::ports::inbound::wheel_cases::ManageWheelCasesUseCase> =
        Arc::new(
            nexus_core::application::wheel_cases_service::WheelCasesService::new(
                wheel_repo.clone(),
            ),
        );
    let service = Arc::new(PlayWheelService::new(
        wheel_repo,
        wallet_repo.clone(),
        bot_config_repo.clone(),
    ));
    let wallet_service = Arc::new(WalletService::new(wallet_repo, bot_config_repo.clone()));
    let coussin_cooldowns: Arc<dyn nexus_core::ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository> =
        Arc::new(crate::adapters::outbound::postgres::coussin_cooldown_repository::PgCoussinCooldownRepository::new(pool.clone()));
    let coussin_repo: Arc<dyn CoussinRepository> = Arc::new(PgCoussinRepository::new(pool.clone()));
    let coussin_profile: Arc<dyn CoussinProfileUseCase> = Arc::new(CoussinService::new(
        coussin_repo,
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

    // ── Game Portal : runtime container (docker | noop) ──
    // NEXUS_GAME_RUNTIME=docker tente le socket Docker ; tout autre valeur
    // (ou l'absence de la variable, ou un socket indisponible) => noop, qui
    // repond Internal sur les operations lifecycle mais laisse le listing
    // et la config fonctionner.
    let runtime_mode = std::env::var("NEXUS_GAME_RUNTIME").unwrap_or_else(|_| "noop".into());
    let container_runtime: Arc<dyn ContainerRuntime> = if runtime_mode == "docker" {
        match make_docker_client() {
            Ok(d) => Arc::new(DockerContainerRuntime::new(d)),
            Err(e) => {
                tracing::warn!(error = %e, "Docker socket indisponible — Game Portal lifecycle inactif (noop)");
                Arc::new(NoopContainerRuntime)
            }
        }
    } else if runtime_mode == "mock" {
        tracing::info!("NEXUS_GAME_RUNTIME=mock — runtime container simule en memoire");
        Arc::new(nexus_core::ports::outbound::game::container_runtime::MockContainerRuntime::new())
    } else {
        tracing::info!("NEXUS_GAME_RUNTIME={runtime_mode} — runtime container noop");
        Arc::new(NoopContainerRuntime)
    };

    let rcon_client: Arc<dyn RconClient> = Arc::new(PooledRconClient::default());

    // Le client redis ne se connecte pas a l'open (lazy) : une URL par defaut
    // ne coute rien tant que l'allocation de port n'est pas sollicitee.
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let redis_client = redis::Client::open(redis_url.as_str())
        .map_err(|e| format!("REDIS_URL invalide ({redis_url}): {e}"))?;
    let port_allocator: Arc<dyn PortAllocator> = Arc::new(RedisPortAllocator::new(redis_client));

    let events: Arc<dyn EventPublisher> = match std::env::var("REDIS_URL") {
        Ok(url) if !url.is_empty() => match RedisEventPublisher::new(&url) {
            Ok(p) => Arc::new(p),
            Err(e) => {
                tracing::warn!(error = %e, "REDIS_URL invalide — events desactives");
                Arc::new(NoopEventPublisher)
            }
        },
        _ => {
            tracing::warn!(
                "REDIS_URL absente — events desactives : le bot ne creera pas \
                 les salons de session game-portal"
            );
            Arc::new(NoopEventPublisher)
        }
    };

    let discord_token = std::env::var("NEXUS_DISCORD_TOKEN").unwrap_or_default();
    let discord_api: Arc<
        dyn nexus_core::ports::outbound::system::discord_api_repository::DiscordApiRepository,
    > = Arc::new(
        crate::adapters::outbound::system::discord_api::ReqwestDiscordApiClient::new(discord_token),
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

    let api_key = std::env::var("NEXUS_API_KEY")
        .ok()
        .filter(|k| !k.is_empty());
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
    if api_key.is_none() {
        tracing::warn!("NEXUS_API_KEY absente — API SANS auth (dev uniquement)");
    }

    Ok(AppState {
        grand_salon,
        play_wheel: service,
        wheel_cases,
        get_wallet: wallet_service.clone(),
        transfer_coins: wallet_service.clone(),
        wallet_history: wallet_service.clone(),
        wallet_leaderboard: wallet_service,
        coussin_profile,
        coussin_combat,
        coussin_inventory,
        coussin_insurance,
        coussin_steal,
        coussin_prime,
        coussin_bet,
        game_servers_uc,
        game_templates_uc,
        game_server_repo,
        game_template_repo,
        game_template_settings_repo,
        game_session_reg_repo,
        game_audit_repo,
        game_session_repo,
        game_container_runtime: container_runtime,
        game_rcon_client: rcon_client,
        game_port_allocator: port_allocator,
        bot_config_repo,
        game_repo,
        events,
        discord_api,
        api_key,
        metrics_token,
        guild_id,
    })
}
