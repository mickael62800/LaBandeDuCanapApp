use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header::AUTHORIZATION, Request, StatusCode};
use platform_api::nexus::adapters::inbound::grpc::NexusGrpcService;
use platform_api::nexus::adapters::inbound::http::{build_router_with, HttpConfig};
use platform_api::nexus::bootstrap::AppState;
use platform_core::nexus::domain::entities::game::server::{
    CreateGameServerCommand, GameServer, GameServerStatus,
};
use platform_core::nexus::domain::entities::game::session::{
    GameSessionRegistration, GameTemplateSettings,
};
use platform_core::nexus::domain::entities::game::template::GameTemplate;
use platform_core::nexus::domain::entities::system::bot_config::{BotDefinition, BotGuildConfig};
use platform_core::nexus::domain::entities::wallet::{Wallet, WalletTransaction};
use platform_core::nexus::domain::entities::wheel::WheelCaseData;
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::inbound::coussin_bet::CoussinBetUseCase;
use platform_core::nexus::ports::inbound::coussin_insurance::CoussinInsuranceUseCase;
use platform_core::nexus::ports::inbound::coussin_inventory::CoussinInventoryUseCase;
use platform_core::nexus::ports::inbound::coussin_prime::CoussinPrimeUseCase;
use platform_core::nexus::ports::inbound::coussin_profile::{
    CoussinCombatUseCase, CoussinProfileUseCase,
};
use platform_core::nexus::ports::inbound::coussin_steal::CoussinStealUseCase;
use platform_core::nexus::ports::inbound::game::manage_game_servers::{
    GameServerDetail, ManageGameServersUseCase,
};
use platform_core::nexus::ports::inbound::game::manage_game_templates::ManageGameTemplatesUseCase;
use platform_core::nexus::ports::inbound::get_wallet::GetWalletUseCase;
use platform_core::nexus::ports::inbound::play_wheel::{
    PlayWheelCommand, PlayWheelResult, PlayWheelUseCase,
};
use platform_core::nexus::ports::inbound::transfer_coins::{
    TransferCoinsCommand, TransferCoinsResult, TransferCoinsUseCase,
};
use platform_core::nexus::ports::inbound::wallet_history::GetWalletHistoryUseCase;
use platform_core::nexus::ports::inbound::wallet_leaderboard::GetWalletLeaderboardUseCase;
use platform_core::nexus::ports::inbound::wheel_cases::{ManageWheelCasesUseCase, WheelCases};
use platform_core::nexus::ports::outbound::casino::game_repository::{
    Game, GamePanel, GameRepository,
};
use platform_core::nexus::ports::outbound::coussin_insurance_repository::CoussinInsurance as OutboundInsurance;
use platform_core::nexus::ports::outbound::coussin_inventory_repository::InventoryItem;
use platform_core::nexus::ports::outbound::coussin_repository::{
    CoussinCombat as OutboundCoussinCombat, CoussinCombatResult as OutboundCombatResult,
    CoussinProfile as OutboundProfile,
};
use platform_core::nexus::ports::outbound::events::EventPublisher;
use platform_core::nexus::ports::outbound::game::container_runtime::{
    ContainerStats, MockContainerRuntime,
};
use platform_core::nexus::ports::outbound::game::game_audit_repository::GameAuditRepository;
use platform_core::nexus::ports::outbound::game::game_server_repository::{
    GameServerRepository, NewGameServer, TemplateUsage,
};
use platform_core::nexus::ports::outbound::game::game_session_repository::{
    GameSessionRegistrationRepository, GameTemplateSettingsRepository,
};
use platform_core::nexus::ports::outbound::game::game_template_repository::GameTemplateRepository;
use platform_core::nexus::ports::outbound::game::player_session_repository::PlayerSessionRepository;
use platform_core::nexus::ports::outbound::game::port_allocator::{PortAllocator, PortKind};
use platform_core::nexus::ports::outbound::game::rcon_client::RconClient;
use platform_core::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository;
use platform_core::nexus::ports::outbound::system::discord_api_repository::DiscordApiRepository;
use platform_proto::nexus::game::v1::game_server_service_server::GameServerService;
use platform_proto::nexus::game::v1::{ExecuteRconRequest, StreamLogsRequest, StreamStatsRequest};
use serde_json::Value;
use tokio_stream::StreamExt;
use tonic::Request as TonicRequest;
use tower::ServiceExt;
use uuid::Uuid;

// ── Mocks conformes aux traits de platform_core::nexus ──

struct DummyPlayWheel;
#[async_trait]
impl PlayWheelUseCase for DummyPlayWheel {
    async fn spin(&self, _: PlayWheelCommand) -> Result<PlayWheelResult, DomainError> {
        todo!()
    }
    async fn can_spin(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
}

struct DummyWheelCases;
#[async_trait]
impl ManageWheelCasesUseCase for DummyWheelCases {
    async fn list(&self, _: &str) -> Result<WheelCases, DomainError> {
        Ok(WheelCases {
            cases: vec![],
            customized: false,
        })
    }
    async fn replace(&self, _: &str, _: Vec<WheelCaseData>) -> Result<WheelCases, DomainError> {
        Ok(WheelCases {
            cases: vec![],
            customized: true,
        })
    }
}

struct DummyWalletUseCase;
#[async_trait]
impl GetWalletUseCase for DummyWalletUseCase {
    async fn get(&self, _guild_id: &str, _user_id: &str) -> Result<Wallet, DomainError> {
        let mut wallet = Wallet::new("g1", "u1");
        wallet.coins = 100;
        Ok(wallet)
    }
}
#[async_trait]
impl TransferCoinsUseCase for DummyWalletUseCase {
    async fn transfer(&self, _: TransferCoinsCommand) -> Result<TransferCoinsResult, DomainError> {
        todo!()
    }
}
#[async_trait]
impl GetWalletHistoryUseCase for DummyWalletUseCase {
    async fn history(
        &self,
        _: &str,
        _: &str,
        _: Option<i64>,
        _: Option<i64>,
    ) -> Result<Vec<WalletTransaction>, DomainError> {
        Ok(vec![])
    }
}
#[async_trait]
impl GetWalletLeaderboardUseCase for DummyWalletUseCase {
    async fn leaderboard(&self, _: &str, _: Option<i64>) -> Result<Vec<Wallet>, DomainError> {
        Ok(vec![])
    }
}

struct DummyCoussinProfile;
#[async_trait]
impl CoussinProfileUseCase for DummyCoussinProfile {
    async fn profile(&self, _: &str, _: &str, _: &str) -> Result<OutboundProfile, DomainError> {
        todo!()
    }
    async fn choose_class(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<OutboundProfile, DomainError> {
        todo!()
    }
    async fn train(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<OutboundProfile, DomainError> {
        todo!()
    }
    async fn combat_history(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Vec<OutboundCombatResult>, DomainError> {
        Ok(vec![])
    }
    async fn ranking(&self, _: &str, _: i64) -> Result<Vec<OutboundProfile>, DomainError> {
        Ok(vec![])
    }
}

struct DummyCoussinCombat;
#[async_trait]
impl CoussinCombatUseCase for DummyCoussinCombat {
    async fn challenge(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<OutboundCoussinCombat, DomainError> {
        todo!()
    }
    async fn accept(&self, _: Uuid, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn refuse(&self, _: Uuid, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn resolve(&self, _: Uuid) -> Result<bool, DomainError> {
        Ok(true)
    }
}

struct DummyCoussinInventory;
#[async_trait]
impl CoussinInventoryUseCase for DummyCoussinInventory {
    async fn inventory(&self, _: &str, _: &str) -> Result<Vec<InventoryItem>, DomainError> {
        Ok(vec![])
    }
    async fn buy(&self, _: &str, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(100)
    }
}

struct DummyCoussinInsurance;
#[async_trait]
impl CoussinInsuranceUseCase for DummyCoussinInsurance {
    async fn buy(&self, _: &str, _: &str) -> Result<OutboundInsurance, DomainError> {
        todo!()
    }
    async fn active(&self, _: &str, _: &str) -> Result<Option<OutboundInsurance>, DomainError> {
        Ok(None)
    }
}

struct DummyCoussinSteal;
#[async_trait]
impl CoussinStealUseCase for DummyCoussinSteal {
    async fn steal(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: bool,
    ) -> Result<platform_core::nexus::ports::inbound::coussin_steal::StealResult, DomainError> {
        todo!()
    }
}

struct DummyCoussinPrime;
#[async_trait]
impl CoussinPrimeUseCase for DummyCoussinPrime {
    async fn place(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

struct DummyCoussinBet;
#[async_trait]
impl CoussinBetUseCase for DummyCoussinBet {
    async fn place(
        &self,
        _: &str,
        _: Uuid,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

struct DummyManageGameServers;
#[async_trait]
impl ManageGameServersUseCase for DummyManageGameServers {
    async fn create(&self, _: CreateGameServerCommand) -> Result<GameServer, DomainError> {
        todo!()
    }
    async fn list_for_guild(&self, _: &str) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn get(&self, _: Uuid) -> Result<GameServerDetail, DomainError> {
        todo!()
    }
    async fn delete(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn start(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn stop(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn restart(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn reveal_ip(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn request_ip_reveal(
        &self,
        _: Uuid,
        _: &str,
    ) -> Result<
        platform_core::nexus::ports::inbound::game::manage_game_servers::RequestIpRevealOutcome,
        DomainError,
    > {
        Ok(
            platform_core::nexus::ports::inbound::game::manage_game_servers::RequestIpRevealOutcome {
                delay_minutes: 10,
                reveal_at: chrono::Utc::now() + chrono::Duration::minutes(10),
                started: true,
            },
        )
    }
    async fn schedule(
        &self,
        _: Uuid,
        _: chrono::DateTime<chrono::Utc>,
        _: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_reveal_schedule(
        &self,
        _: Uuid,
        _: Option<chrono::DateTime<chrono::Utc>>,
        _: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_logs(&self, _: Uuid, _: u32) -> Result<Vec<String>, DomainError> {
        Ok(vec!["[Server] Started".into()])
    }
    async fn get_stats(&self, _: Uuid) -> Result<ContainerStats, DomainError> {
        Ok(ContainerStats {
            cpu_percent: 12.5,
            memory_used_bytes: 1024,
            memory_limit_bytes: 2048,
            network_rx_bytes: 100,
            network_tx_bytes: 200,
        })
    }
    async fn update_config(
        &self,
        _: Uuid,
        _: std::collections::HashMap<String, String>,
        _: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn execute_rcon(&self, _: Uuid, _: &str, _: &str) -> Result<String, DomainError> {
        Ok("OK".into())
    }
}

struct DummyManageGameTemplates;
#[async_trait]
impl ManageGameTemplatesUseCase for DummyManageGameTemplates {
    async fn list_for_guild(&self, _: &str) -> Result<Vec<GameTemplate>, DomainError> {
        Ok(vec![])
    }
    async fn get(&self, _: Uuid) -> Result<GameTemplate, DomainError> {
        todo!()
    }
    async fn get_by_slug(&self, _: &str) -> Result<GameTemplate, DomainError> {
        todo!()
    }
}

struct DummyGameServerRepo;
#[async_trait]
impl GameServerRepository for DummyGameServerRepo {
    async fn create(&self, _: NewGameServer) -> Result<GameServer, DomainError> {
        todo!()
    }
    async fn find_by_id(&self, _: Uuid) -> Result<Option<GameServer>, DomainError> {
        Ok(None)
    }
    async fn list_by_guild(&self, _: &str) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn list_running(&self) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn list_active(&self) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn update_runtime(
        &self,
        _: Uuid,
        _: platform_core::nexus::ports::outbound::game::game_server_repository::GameServerRuntimeUpdate,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_status(
        &self,
        _: Uuid,
        _: GameServerStatus,
        _: Option<&str>,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn try_transition_status(
        &self,
        _: Uuid,
        _: &[GameServerStatus],
        _: GameServerStatus,
    ) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn update_player_activity(&self, _: Uuid, _: i32) -> Result<(), DomainError> {
        Ok(())
    }
    async fn record_restart_attempt(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn reset_restart_attempts(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn soft_delete(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn count_active_for_guild(&self, _: &str) -> Result<(i32, i32), DomainError> {
        Ok((0, 0))
    }
    async fn template_usages(
        &self,
        _: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, TemplateUsage>, DomainError> {
        Ok(std::collections::HashMap::new())
    }
    async fn set_session_channels(
        &self,
        _: Uuid,
        _: Option<&str>,
        _: Option<&str>,
    ) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn mark_ip_revealed(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_ip_reveal_due(&self) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn list_awaiting_reveal_no_ping_today(&self) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_ip_reveal_at(
        &self,
        _: Uuid,
        _: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_scheduled_due_to_start(&self) -> Result<Vec<GameServer>, DomainError> {
        Ok(vec![])
    }
}

struct DummyGameTemplateRepo;
#[async_trait]
impl GameTemplateRepository for DummyGameTemplateRepo {
    async fn list(&self) -> Result<Vec<GameTemplate>, DomainError> {
        Ok(vec![])
    }
    async fn find_by_id(&self, _: Uuid) -> Result<Option<GameTemplate>, DomainError> {
        Ok(None)
    }
    async fn find_by_slug(&self, _: &str) -> Result<Option<GameTemplate>, DomainError> {
        Ok(None)
    }
}

struct DummyGameTemplateSettingsRepo;
#[async_trait]
impl GameTemplateSettingsRepository for DummyGameTemplateSettingsRepo {
    async fn get(&self, _: &str, _: &str) -> Result<Option<GameTemplateSettings>, DomainError> {
        Ok(None)
    }
    async fn list(&self, _: &str) -> Result<Vec<GameTemplateSettings>, DomainError> {
        Ok(vec![])
    }
    async fn set_role(&self, _: &str, _: &str, _: Option<&str>) -> Result<(), DomainError> {
        Ok(())
    }
}

struct DummySessionRegRepo;
#[async_trait]
impl GameSessionRegistrationRepository for DummySessionRegRepo {
    async fn register(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn unregister(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list(&self, _: Uuid) -> Result<Vec<GameSessionRegistration>, DomainError> {
        Ok(vec![])
    }
}

struct DummyAuditRepo;
#[async_trait]
impl GameAuditRepository for DummyAuditRepo {
    async fn log(
        &self,
        _: &str,
        _: Option<Uuid>,
        _: Option<&str>,
        _: platform_core::nexus::domain::entities::game::audit::GameAuditAction,
        _: serde_json::Value,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_for_server(
        &self,
        _: Uuid,
        _: i64,
    ) -> Result<Vec<platform_core::nexus::domain::entities::game::audit::GameAuditEntry>, DomainError>
    {
        Ok(vec![])
    }
    async fn list_for_guild(
        &self,
        _: &str,
        _: i64,
        _: i64,
    ) -> Result<Vec<platform_core::nexus::domain::entities::game::audit::GameAuditEntry>, DomainError>
    {
        Ok(vec![])
    }
}

struct DummyPlayerSessionRepo;
#[async_trait]
impl PlayerSessionRepository for DummyPlayerSessionRepo {
    async fn open(&self, _: Uuid, _: &str) -> Result<Uuid, DomainError> {
        Ok(Uuid::new_v4())
    }
    async fn close(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_active(
        &self,
        _: Uuid,
    ) -> Result<
        Vec<platform_core::nexus::domain::entities::game::player_session::PlayerSession>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn list_history(
        &self,
        _: Uuid,
        _: i64,
        _: i64,
    ) -> Result<
        Vec<platform_core::nexus::domain::entities::game::player_session::PlayerSession>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn close_all_active(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
}

struct DummyRconClient;
#[async_trait]
impl RconClient for DummyRconClient {
    async fn execute(
        &self,
        _: &platform_core::nexus::ports::outbound::game::rcon_client::RconConnectionParams,
        _: &str,
    ) -> Result<platform_core::nexus::ports::outbound::game::rcon_client::RconResponse, DomainError>
    {
        todo!()
    }
}

struct DummyPortAllocator;
#[async_trait]
impl PortAllocator for DummyPortAllocator {
    async fn allocate(&self, _: PortKind, _: u16, _: u16, _: &str) -> Result<u16, DomainError> {
        Ok(25565)
    }
    async fn release(&self, _: PortKind, _: u16) -> Result<(), DomainError> {
        Ok(())
    }
    async fn is_available(&self, _: PortKind, _: u16) -> Result<bool, DomainError> {
        Ok(true)
    }
}

struct DummyBotConfigRepo;
#[async_trait]
impl BotConfigRepository for DummyBotConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
        Ok(vec![])
    }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

struct DummyGameRepo;
#[async_trait]
impl GameRepository for DummyGameRepo {
    async fn list(&self, _: &str) -> Result<Vec<Game>, DomainError> {
        Ok(vec![])
    }
    async fn list_by_category(&self, _: &str, _: Option<&str>) -> Result<Vec<Game>, DomainError> {
        Ok(vec![])
    }
    async fn create(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<&str>,
    ) -> Result<Game, DomainError> {
        todo!()
    }
    async fn update(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<Option<&str>>,
        _: Option<Option<&str>>,
    ) -> Result<Option<Game>, DomainError> {
        todo!()
    }
    async fn delete(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn find_by_name(&self, _: &str, _: &str) -> Result<Option<Game>, DomainError> {
        Ok(None)
    }
    async fn set_role_id(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<Option<Game>, DomainError> {
        Ok(None)
    }
    async fn save_panel(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<GamePanel, DomainError> {
        todo!()
    }
    async fn find_panel_by_message(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<GamePanel>, DomainError> {
        Ok(None)
    }
    async fn list_panels(&self, _: &str) -> Result<Vec<GamePanel>, DomainError> {
        Ok(vec![])
    }
}

struct DummyEventPublisher;
#[async_trait]
impl EventPublisher for DummyEventPublisher {
    async fn publish(&self, _: &str, _: serde_json::Value) {}
}

struct DummyDiscordApi;
#[async_trait]
impl DiscordApiRepository for DummyDiscordApi {
    async fn upload_emoji(
        &self,
        _: &str,
        _: &str,
        _: &[u8],
        _: &str,
    ) -> Result<(String, String), DomainError> {
        Ok(("e1".into(), "emoji".into()))
    }
}

/// Depot du Grand Salon non exerce par ces tests : aucune des routes couvertes
/// ici ne l'atteint. Chaque methode panique donc plutot que de mentir avec une
/// valeur vide — si un test futur touche le Grand Salon, il doit le voir tout
/// de suite et brancher un vrai double.
struct DummyGrandSalonRepo;

#[async_trait::async_trait]
impl platform_core::nexus::ports::outbound::grand_salon_repository::GrandSalonRepository
    for DummyGrandSalonRepo
{
    async fn find_habitue(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        Option<platform_core::nexus::domain::entities::grand_salon::Habitué>,
        platform_core::nexus::domain::errors::DomainError,
    > {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn save_habitue(
        &self,
        _: &platform_core::nexus::domain::entities::grand_salon::Habitué,
    ) -> Result<(), platform_core::nexus::domain::errors::DomainError> {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn claim_daily(
        &self,
        _: uuid::Uuid,
    ) -> Result<bool, platform_core::nexus::domain::errors::DomainError> {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn create_cercle(
        &self,
        _: &platform_core::nexus::domain::entities::grand_salon::Cercle,
    ) -> Result<(), platform_core::nexus::domain::errors::DomainError> {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn list_cercles(
        &self,
        _: &str,
    ) -> Result<
        Vec<platform_core::nexus::domain::entities::grand_salon::Cercle>,
        platform_core::nexus::domain::errors::DomainError,
    > {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn create_motion(
        &self,
        _: &platform_core::nexus::domain::entities::grand_salon::MotionDuSalon,
    ) -> Result<(), platform_core::nexus::domain::errors::DomainError> {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn list_motions(
        &self,
        _: &str,
    ) -> Result<
        Vec<platform_core::nexus::domain::entities::grand_salon::MotionDuSalon>,
        platform_core::nexus::domain::errors::DomainError,
    > {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn cast_vote(
        &self,
        _: uuid::Uuid,
        _: uuid::Uuid,
        _: bool,
        _: i64,
    ) -> Result<(), platform_core::nexus::domain::errors::DomainError> {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn vote_totals(
        &self,
        _: uuid::Uuid,
    ) -> Result<(i64, i64), platform_core::nexus::domain::errors::DomainError> {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn due_motions(
        &self,
    ) -> Result<
        Vec<platform_core::nexus::domain::entities::grand_salon::MotionDuSalon>,
        platform_core::nexus::domain::errors::DomainError,
    > {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn close_motion(
        &self,
        _: uuid::Uuid,
        _: bool,
    ) -> Result<(), platform_core::nexus::domain::errors::DomainError> {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn publish_gazette(
        &self,
        _: &platform_core::nexus::domain::entities::grand_salon::GazetteArticle,
    ) -> Result<(), platform_core::nexus::domain::errors::DomainError> {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn list_gazette(
        &self,
        _: &str,
    ) -> Result<
        Vec<platform_core::nexus::domain::entities::grand_salon::GazetteArticle>,
        platform_core::nexus::domain::errors::DomainError,
    > {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn create_dossier(
        &self,
        _: &platform_core::nexus::domain::entities::grand_salon::Dossier,
    ) -> Result<(), platform_core::nexus::domain::errors::DomainError> {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn list_dossiers(
        &self,
        _: &str,
        _: uuid::Uuid,
    ) -> Result<
        Vec<platform_core::nexus::domain::entities::grand_salon::Dossier>,
        platform_core::nexus::domain::errors::DomainError,
    > {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
    async fn reveal_dossier(
        &self,
        _: uuid::Uuid,
        _: uuid::Uuid,
    ) -> Result<(), platform_core::nexus::domain::errors::DomainError> {
        unimplemented!("Grand Salon non exerce par ces tests")
    }
}

/// Cle des tests. `nexus-api` n'a plus de mode « sans jeton » : le bootstrap
/// refuse de demarrer sans cle d'au moins 16 caracteres, et l'etat la porte en
/// `String`. Les tests exercent donc la meme posture que la production.
const TEST_API_KEY: &str = "cle-de-test-nexus-32-caracteres";

fn create_test_app_state(api_key: impl Into<String>) -> AppState {
    let api_key = api_key.into();
    AppState {
        job_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/nexus_test")
            .unwrap(),
        grand_salon: Arc::new(
            platform_core::nexus::application::grand_salon_service::GrandSalonService::new(
                Arc::new(DummyGrandSalonRepo),
                1_000,
            ),
        ),
        play_wheel: Arc::new(DummyPlayWheel),
        wheel_cases: Arc::new(DummyWheelCases),
        get_wallet: Arc::new(DummyWalletUseCase),
        transfer_coins: Arc::new(DummyWalletUseCase),
        wallet_history: Arc::new(DummyWalletUseCase),
        wallet_leaderboard: Arc::new(DummyWalletUseCase),
        coussin_profile: Arc::new(DummyCoussinProfile),
        coussin_combat: Arc::new(DummyCoussinCombat),
        coussin_inventory: Arc::new(DummyCoussinInventory),
        coussin_insurance: Arc::new(DummyCoussinInsurance),
        coussin_steal: Arc::new(DummyCoussinSteal),
        coussin_prime: Arc::new(DummyCoussinPrime),
        coussin_bet: Arc::new(DummyCoussinBet),
        game_servers_uc: Arc::new(DummyManageGameServers),
        game_templates_uc: Arc::new(DummyManageGameTemplates),
        game_server_repo: Arc::new(DummyGameServerRepo),
        game_template_repo: Arc::new(DummyGameTemplateRepo),
        game_template_settings_repo: Arc::new(DummyGameTemplateSettingsRepo),
        game_session_reg_repo: Arc::new(DummySessionRegRepo),
        game_audit_repo: Arc::new(DummyAuditRepo),
        game_session_repo: Arc::new(DummyPlayerSessionRepo),
        game_container_runtime: Arc::new(MockContainerRuntime::new()),
        game_rcon_client: Arc::new(DummyRconClient),
        game_port_allocator: Arc::new(DummyPortAllocator),
        bot_config_repo: Arc::new(DummyBotConfigRepo),
        game_repo: Arc::new(DummyGameRepo),
        events: Arc::new(DummyEventPublisher),
        discord_api: Arc::new(DummyDiscordApi),
        api_key,
        metrics_token: None,
        guild_id: None,
    }
}

fn setup_nexus_router(api_key: impl Into<String>) -> axum::Router {
    static TEST_INIT: std::sync::Once = std::sync::Once::new();
    TEST_INIT.call_once(platform_api::shared::metrics::init_prometheus);
    let state = create_test_app_state(api_key);
    let mut config = HttpConfig::from_env();
    config.rate_limit_per_sec = 1000;
    config.heavy_rate_limit_per_sec = 1000;
    build_router_with(state, config)
}

/// Requete authentifiee : le Bearer est pose par defaut, sinon chaque test
/// mesurerait la posture d'authentification au lieu de ce qu'il vise.
/// Pour l'exercer explicitement, voir `create_request_sans_auth`.
fn create_request(method: &str, uri: &str, body: Body) -> Request<Body> {
    let mut req = create_request_sans_auth(method, uri, body);
    req.headers_mut().insert(
        AUTHORIZATION,
        format!("Bearer {TEST_API_KEY}").parse().unwrap(),
    );
    req
}

fn create_request_sans_auth(method: &str, uri: &str, body: Body) -> Request<Body> {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .unwrap();

    let dummy_addr: std::net::SocketAddr = "127.0.0.1:12345".parse().unwrap();
    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(dummy_addr));
    req
}

#[tokio::test]
async fn test_nexus_health_endpoint() {
    let app = setup_nexus_router(TEST_API_KEY);
    let req = create_request("GET", "/health", Body::empty());

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_nexus_wallet_endpoint() {
    let app = setup_nexus_router(TEST_API_KEY);
    let req = create_request("GET", "/api/wallet/guild123/user456", Body::empty());

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["guild_id"], "g1");
    assert_eq!(json["user_id"], "u1");
    assert_eq!(json["coins"], 100);
}

#[tokio::test]
async fn test_nexus_auth_bearer_toujours_exige() {
    // Sans token -> 401. Il n'existe plus de configuration ou cette requete
    // passerait : c'est ce que ce test verrouille.
    let app = setup_nexus_router(TEST_API_KEY);
    let req = create_request_sans_auth("GET", "/api/wallet/guild123/user456", Body::empty());
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Mauvais token -> 401 egalement.
    let app = setup_nexus_router(TEST_API_KEY);
    let mut req = create_request_sans_auth("GET", "/api/wallet/guild123/user456", Body::empty());
    req.headers_mut()
        .insert(AUTHORIZATION, "Bearer mauvaise-cle".parse().unwrap());
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Avec le bon token -> 200 OK
    let app = setup_nexus_router(TEST_API_KEY);
    let req = create_request("GET", "/api/wallet/guild123/user456", Body::empty());
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_nexus_bot_definitions_endpoint() {
    let app = setup_nexus_router(TEST_API_KEY);
    let req = create_request("GET", "/api/bots/definitions", Body::empty());

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_nexus_grpc_service_logs_and_stats_stream() {
    let state = create_test_app_state(TEST_API_KEY);
    let grpc_service = NexusGrpcService {
        game_servers_uc: state.game_servers_uc.clone(),
    };

    let id = Uuid::new_v4().to_string();
    let logs_req = TonicRequest::new(StreamLogsRequest {
        server_id: id.clone(),
        tail_lines: 10,
        follow: false,
    });

    let res = grpc_service.stream_logs(logs_req).await.unwrap();
    let mut stream = res.into_inner();

    let chunk = stream.next().await.unwrap().unwrap();
    assert_eq!(chunk.server_id, id);
    assert_eq!(chunk.line, "[Server] Started");

    let stats_req = TonicRequest::new(StreamStatsRequest {
        server_id: id.clone(),
    });
    let res_stats = grpc_service.stream_stats(stats_req).await.unwrap();
    let mut stats_stream = res_stats.into_inner();

    let stat_item = stats_stream.next().await.unwrap().unwrap();
    assert_eq!(stat_item.server_id, id);
    assert_eq!(stat_item.cpu_percentage, 12.5);
    assert_eq!(stat_item.memory_used_bytes, 1024);
}

#[tokio::test]
async fn test_nexus_grpc_service_execute_rcon() {
    let state = create_test_app_state(TEST_API_KEY);
    let grpc_service = NexusGrpcService {
        game_servers_uc: state.game_servers_uc.clone(),
    };

    let id = Uuid::new_v4().to_string();
    let rcon_req = TonicRequest::new(ExecuteRconRequest {
        server_id: id.clone(),
        command: "list".into(),
    });

    let res = grpc_service.execute_rcon(rcon_req).await.unwrap();
    let inner = res.into_inner();
    assert_eq!(inner.server_id, id);
    assert_eq!(inner.response, "OK");
    assert!(inner.success);
}

#[tokio::test]
async fn test_nexus_sse_stream_logs_endpoint() {
    let app = setup_nexus_router(TEST_API_KEY);
    let id = Uuid::new_v4();
    let uri = format!("/api/games/servers/{id}/stream-logs?lines=10");
    let req = create_request("GET", &uri, Body::empty());

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/event-stream"));
}

#[tokio::test]
async fn test_nexus_sse_stream_stats_endpoint() {
    let app = setup_nexus_router(TEST_API_KEY);
    let id = Uuid::new_v4();
    let uri = format!("/api/games/servers/{id}/stream-stats");
    let req = create_request("GET", &uri, Body::empty());

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/event-stream"));
}
