use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::application::system::reset_guild_service::ResetGuildService;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::system::reset_guild::ResetGuildUseCase;
use crate::sentinel::ports::outbound::system::guild_reset_repository::{
    GuildResetRepository, ResetDiscordContext,
};

struct MockResetRepo {
    /// Nom du serveur renvoye par `guild_name` (`None` = serveur inconnu).
    name: Option<String>,
    wiped: AtomicBool,
}

impl MockResetRepo {
    fn new(name: Option<&str>) -> Self {
        Self {
            name: name.map(str::to_string),
            wiped: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl GuildResetRepository for MockResetRepo {
    async fn guild_name(&self, _guild_id: &str) -> Result<Option<String>, DomainError> {
        Ok(self.name.clone())
    }

    async fn collect_discord_context(
        &self,
        _guild_id: &str,
    ) -> Result<ResetDiscordContext, DomainError> {
        Ok(ResetDiscordContext {
            quarantine_role_id: Some("q-role".into()),
            temp_role_ids: vec!["t1".into(), "t2".into()],
        })
    }

    async fn wipe_guild(&self, _guild_id: &str) -> Result<Vec<(String, u64)>, DomainError> {
        self.wiped.store(true, Ordering::SeqCst);
        Ok(vec![
            ("infractions".into(), 12),
            ("user_levels".into(), 30),
            ("bot_guild_config".into(), 0),
        ])
    }
}

const GUILD: &str = "123456789012345678";

#[tokio::test]
async fn reset_ok_with_exact_name_sums_rows_and_keeps_context() {
    let repo = Arc::new(MockResetRepo::new(Some("Mon Serveur")));
    let svc = ResetGuildService::new(repo.clone());

    let out = svc.reset(GUILD, "Mon Serveur").await.expect("reset ok");
    assert!(repo.wiped.load(Ordering::SeqCst));
    assert_eq!(out.tables_wiped.len(), 3);
    assert_eq!(out.total_rows, 42);
    assert_eq!(
        out.discord_context.quarantine_role_id.as_deref(),
        Some("q-role")
    );
    assert_eq!(out.discord_context.temp_role_ids, vec!["t1", "t2"]);
}

#[tokio::test]
async fn reset_trims_confirmation_whitespace() {
    let repo = Arc::new(MockResetRepo::new(Some("Mon Serveur")));
    let svc = ResetGuildService::new(repo.clone());

    svc.reset(GUILD, "  Mon Serveur \n")
        .await
        .expect("reset ok");
    assert!(repo.wiped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn reset_wrong_confirmation_is_forbidden_and_does_not_wipe() {
    let repo = Arc::new(MockResetRepo::new(Some("Mon Serveur")));
    let svc = ResetGuildService::new(repo.clone());

    let err = svc.reset(GUILD, "mon serveur").await.unwrap_err();
    assert!(matches!(err, DomainError::Forbidden(_)), "{err:?}");
    assert!(
        !repo.wiped.load(Ordering::SeqCst),
        "wipe ne doit PAS etre appele"
    );
}

#[tokio::test]
async fn reset_unknown_guild_is_not_found_and_does_not_wipe() {
    let repo = Arc::new(MockResetRepo::new(None));
    let svc = ResetGuildService::new(repo.clone());

    let err = svc.reset(GUILD, "peu importe").await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)), "{err:?}");
    assert!(!repo.wiped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn reset_empty_guild_id_is_rejected_before_any_db_access() {
    let repo = Arc::new(MockResetRepo::new(Some("Mon Serveur")));
    let svc = ResetGuildService::new(repo.clone());

    let err = svc.reset("   ", "Mon Serveur").await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)), "{err:?}");
    assert!(!repo.wiped.load(Ordering::SeqCst));
}
