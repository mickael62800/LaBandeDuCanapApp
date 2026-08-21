//! Tests du service PlayWheel sur mocks (pas de DB).

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::nexus::application::play_wheel_service::PlayWheelService;
use crate::nexus::domain::entities::wallet::Wallet;
use crate::nexus::domain::entities::wallet::WalletMutation;
use crate::nexus::domain::entities::wheel::WheelSpin;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::get_wallet::GetWalletUseCase;
use crate::nexus::ports::inbound::play_wheel::PlayWheelCommand;
use crate::nexus::ports::inbound::play_wheel::PlayWheelUseCase;
use crate::nexus::ports::outbound::wallet_repository::WalletRepository;
use crate::nexus::ports::outbound::wheel_repository::WheelRepository;

// ── Mocks ──

#[derive(Default)]
struct MockWheelRepo {
    /// Valeur retournee par try_claim_today.
    claim_ok: bool,
    logged: Mutex<Vec<WheelSpin>>,
    wallet_repo: Mutex<Option<Arc<MockWalletRepo>>>,
}

#[async_trait]
impl WheelRepository for MockWheelRepo {
    async fn try_claim(&self, _g: &str, _u: &str, _h: i64) -> Result<bool, DomainError> {
        Ok(self.claim_ok)
    }
    /// Le mock decrit un joueur qui n'a pas encore tire quand le claim est
    /// possible : les deux informations decrivent le meme etat.
    async fn has_claimed_recently(&self, _g: &str, _u: &str, _h: i64) -> Result<bool, DomainError> {
        Ok(!self.claim_ok)
    }
    async fn log_spin(&self, spin: &WheelSpin) -> Result<(), DomainError> {
        self.logged.lock().unwrap().push(spin.clone());
        Ok(())
    }
    /// Aucune case personnalisee : le service doit retomber sur la roue
    /// historique. C'est le cas de tous les serveurs qui n'ouvriront jamais
    /// l'editeur, donc celui qu'il faut couvrir par defaut.
    async fn list_cases(
        &self,
        _g: &str,
    ) -> Result<Vec<crate::nexus::domain::entities::wheel::WheelCaseData>, DomainError> {
        Ok(vec![])
    }
    async fn replace_cases(
        &self,
        _g: &str,
        _cases: &[crate::nexus::domain::entities::wheel::WheelCaseData],
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn execute_spin_transaction(
        &self,
        _guild_id: &str,
        _user_id: &str,
        _cooldown_hours: i64,
        spin: &WheelSpin,
        wallet: &crate::nexus::domain::entities::wallet::Wallet,
        mutation: Option<&crate::nexus::domain::entities::wallet::WalletMutation>,
    ) -> Result<bool, DomainError> {
        if self.claim_ok {
            self.logged.lock().unwrap().push(spin.clone());
            if let Some(w_repo) = self.wallet_repo.lock().unwrap().as_ref() {
                if let Some(mutat) = mutation {
                    w_repo
                        .saved
                        .lock()
                        .unwrap()
                        .push((wallet.clone(), mutat.clone()));
                }
            }
        }
        Ok(self.claim_ok)
    }
}

#[derive(Default)]
struct MockWalletRepo {
    initial_coins: i64,
    saved: Mutex<Vec<(Wallet, WalletMutation)>>,
}

#[async_trait]
impl WalletRepository for MockWalletRepo {
    async fn find(&self, g: &str, u: &str) -> Result<Option<Wallet>, DomainError> {
        // Le wallet "existe" toujours : pas de credit de solde de depart,
        // le comportement wheel historique des tests est conserve.
        let mut w = Wallet::new(g, u);
        if self.initial_coins > 0 {
            w.credit(self.initial_coins)?;
        }
        Ok(Some(w))
    }
    async fn starting_coins(&self, _g: &str) -> Result<Option<i64>, DomainError> {
        Ok(None)
    }
    async fn save_with_transaction(
        &self,
        wallet: &Wallet,
        mutation: &WalletMutation,
    ) -> Result<(), DomainError> {
        self.saved
            .lock()
            .unwrap()
            .push((wallet.clone(), mutation.clone()));
        Ok(())
    }
    async fn transfer_atomic(
        &self,
        _g: &str,
        _f: &str,
        _t: &str,
        _a: i64,
        _r: Option<&str>,
    ) -> Result<crate::nexus::ports::outbound::wallet_repository::TransferOutcome, DomainError>
    {
        unimplemented!("pas de transfert dans les tests wheel")
    }
    async fn history(
        &self,
        _g: &str,
        _u: &str,
        _l: i64,
        _o: i64,
    ) -> Result<Vec<crate::nexus::domain::entities::wallet::WalletTransaction>, DomainError> {
        unimplemented!("pas d'historique dans les tests wheel")
    }
    async fn leaderboard(&self, _g: &str, _l: i64) -> Result<Vec<Wallet>, DomainError> {
        unimplemented!("pas de leaderboard dans les tests wheel")
    }
}

fn service(
    claim_ok: bool,
    initial_coins: i64,
) -> (PlayWheelService, Arc<MockWheelRepo>, Arc<MockWalletRepo>) {
    let wheel = Arc::new(MockWheelRepo {
        claim_ok,
        ..Default::default()
    });
    let wallet = Arc::new(MockWalletRepo {
        initial_coins,
        ..Default::default()
    });
    *wheel.wallet_repo.lock().unwrap() = Some(wallet.clone());
    (
        PlayWheelService::new(
            wheel.clone(),
            wallet.clone(),
            Arc::new(crate::nexus::application::economy_config::EnabledBotConfigRepository),
        ),
        wheel,
        wallet,
    )
}

fn cmd() -> PlayWheelCommand {
    PlayWheelCommand {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        username: "Alice".into(),
    }
}

// ── Tests ──

#[tokio::test]
async fn spin_rejected_when_daily_already_claimed() {
    let (svc, wheel, wallet) = service(false, 1000);
    let err = svc.spin(cmd()).await.unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
    assert!(wheel.logged.lock().unwrap().is_empty(), "aucun spin logge");
    assert!(wallet.saved.lock().unwrap().is_empty(), "wallet intact");
}

#[tokio::test]
async fn spin_logs_exactly_one_spin_with_command_identity() {
    let (svc, wheel, _) = service(true, 1000);
    let res = svc.spin(cmd()).await.unwrap();
    let logged = wheel.logged.lock().unwrap();
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].guild_id, "g1");
    assert_eq!(logged[0].user_id, "u1");
    assert_eq!(logged[0].username, "Alice");
    assert_eq!(logged[0].id, res.spin.id);
    assert_eq!(logged[0].payout, res.spin.payout);
}

#[tokio::test]
async fn wallet_mutation_matches_payout_sign_and_balance() {
    // 200 spins : couvre statistiquement gains, pertes et blanche.
    for _ in 0..200 {
        let (svc, _, wallet) = service(true, 1000);
        let res = svc.spin(cmd()).await.unwrap();
        let saved = wallet.saved.lock().unwrap();
        let payout = res.spin.payout;
        if payout > 0 {
            assert_eq!(saved.len(), 1);
            assert_eq!(saved[0].1.amount, payout);
            assert_eq!(saved[0].1.source, "wheel_payout");
            assert_eq!(res.balance_after, 1000 + payout);
        } else if payout < 0 {
            assert_eq!(saved.len(), 1);
            assert_eq!(saved[0].1.source, "wheel_loss");
            // Debit clampe : solde initial 1000, bombe -2000 -> solde 0.
            assert_eq!(res.balance_after, (1000 + payout).max(0));
            assert_eq!(saved[0].1.amount, -(1000i64.min(-payout)));
        } else {
            assert!(saved.is_empty(), "blanche : aucune mutation wallet");
            assert_eq!(res.balance_after, 1000);
        }
        assert_eq!(
            saved
                .iter()
                .map(|s| s.1.balance_after)
                .next_back()
                .unwrap_or(1000),
            res.balance_after
        );
    }
}

#[tokio::test]
async fn loss_on_empty_wallet_never_goes_negative() {
    for _ in 0..300 {
        let (svc, _, wallet) = service(true, 0);
        let res = svc.spin(cmd()).await.unwrap();
        assert!(res.balance_after >= 0);
        if res.spin.payout < 0 {
            // Rien a debiter -> aucune transaction journalisee.
            assert!(wallet.saved.lock().unwrap().is_empty());
            assert_eq!(res.balance_after, 0);
        }
    }
}

#[tokio::test]
async fn memorable_flag_follows_case_key() {
    for _ in 0..300 {
        let (svc, _, _) = service(true, 1000);
        let res = svc.spin(cmd()).await.unwrap();
        let expected = matches!(res.spin.case_key.as_str(), "jackpot" | "licorne" | "bombe");
        assert_eq!(res.is_memorable, expected);
    }
}

#[tokio::test]
async fn get_wallet_returns_repo_state() {
    let (svc, _, _) = service(true, 750);
    let w = svc.get("g1", "u1").await.unwrap();
    assert_eq!(w.coins, 750);
    assert_eq!(w.guild_id, "g1");
    assert_eq!(w.user_id, "u1");
}

#[tokio::test]
async fn test_can_spin() {
    let (svc_can, _, _) = service(true, 1000);
    assert!(svc_can.can_spin("g1", "u1").await.unwrap());

    let (svc_cant, _, _) = service(false, 1000);
    assert!(!svc_cant.can_spin("g1", "u1").await.unwrap());
}

struct MockWheelRepoTxFail {
    logged: Mutex<Vec<WheelSpin>>,
}

#[async_trait]
impl WheelRepository for MockWheelRepoTxFail {
    async fn try_claim(&self, _g: &str, _u: &str, _h: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn has_claimed_recently(&self, _g: &str, _u: &str, _h: i64) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn log_spin(&self, _spin: &WheelSpin) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_cases(
        &self,
        _g: &str,
    ) -> Result<Vec<crate::nexus::domain::entities::wheel::WheelCaseData>, DomainError> {
        Ok(vec![])
    }
    async fn replace_cases(
        &self,
        _g: &str,
        _cases: &[crate::nexus::domain::entities::wheel::WheelCaseData],
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn execute_spin_transaction(
        &self,
        _guild_id: &str,
        _user_id: &str,
        _cooldown_hours: i64,
        _spin: &WheelSpin,
        _wallet: &crate::nexus::domain::entities::wallet::Wallet,
        _mutation: Option<&crate::nexus::domain::entities::wallet::WalletMutation>,
    ) -> Result<bool, DomainError> {
        Ok(false) // Simule l'échec de la transaction (déjà claimé par un autre thread)
    }
}

#[tokio::test]
async fn test_spin_concurrent_transaction_fails() {
    let wheel = Arc::new(MockWheelRepoTxFail {
        logged: Mutex::new(vec![]),
    });
    let wallet = Arc::new(MockWalletRepo {
        initial_coins: 1000,
        saved: Mutex::new(vec![]),
    });
    let svc = PlayWheelService::new(
        wheel.clone(),
        wallet.clone(),
        Arc::new(crate::nexus::application::economy_config::EnabledBotConfigRepository),
    );
    let err = svc.spin(cmd()).await.unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
}

#[tokio::test]
async fn test_spin_with_wheel_disabled() {
    struct MockConfigRepoWheelDisabled;
    #[async_trait::async_trait]
    impl crate::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository
        for MockConfigRepoWheelDisabled
    {
        async fn get_definitions(
            &self,
        ) -> Result<
            Vec<crate::nexus::domain::entities::system::bot_config::BotDefinition>,
            crate::nexus::domain::errors::DomainError,
        > {
            Ok(vec![])
        }
        async fn get_config(
            &self,
            _guild_id: &str,
            bot_name: &str,
        ) -> Result<Vec<crate::nexus::domain::entities::system::bot_config::BotGuildConfig>, crate::nexus::domain::errors::DomainError> {
            if bot_name == "nexus-economy" {
                Ok(vec![crate::nexus::domain::entities::system::bot_config::BotGuildConfig {
                    id: uuid::Uuid::nil(),
                    guild_id: "g1".into(),
                    bot_name: bot_name.to_string(),
                    config_key: "wheel_enabled".into(),
                    config_value: "false".into(),
                    updated_at: chrono::Utc::now(),
                }])
            } else {
                Ok(vec![])
            }
        }
        async fn get_all_config(
            &self,
            _: &str,
        ) -> Result<Vec<crate::nexus::domain::entities::system::bot_config::BotGuildConfig>, crate::nexus::domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), crate::nexus::domain::errors::DomainError> {
            Ok(())
        }
        async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), crate::nexus::domain::errors::DomainError> {
            Ok(())
        }
    }

    let wheel = Arc::new(MockWheelRepo {
        claim_ok: true,
        logged: Mutex::new(vec![]),
        wallet_repo: Mutex::new(None),
    });
    let wallet = Arc::new(MockWalletRepo {
        initial_coins: 1000,
        saved: Mutex::new(vec![]),
    });
    let svc_disabled = PlayWheelService::new(
        wheel.clone(),
        wallet.clone(),
        Arc::new(MockConfigRepoWheelDisabled),
    );

    let err = svc_disabled.spin(cmd()).await.unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
}

#[tokio::test]
async fn test_can_spin_with_wheel_disabled() {
    struct MockConfigRepoWheelDisabled;
    #[async_trait::async_trait]
    impl crate::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository
        for MockConfigRepoWheelDisabled
    {
        async fn get_definitions(
            &self,
        ) -> Result<
            Vec<crate::nexus::domain::entities::system::bot_config::BotDefinition>,
            crate::nexus::domain::errors::DomainError,
        > {
            Ok(vec![])
        }
        async fn get_config(
            &self,
            _guild_id: &str,
            bot_name: &str,
        ) -> Result<Vec<crate::nexus::domain::entities::system::bot_config::BotGuildConfig>, crate::nexus::domain::errors::DomainError> {
            if bot_name == "nexus-economy" {
                Ok(vec![crate::nexus::domain::entities::system::bot_config::BotGuildConfig {
                    id: uuid::Uuid::nil(),
                    guild_id: "g1".into(),
                    bot_name: bot_name.to_string(),
                    config_key: "wheel_enabled".into(),
                    config_value: "false".into(),
                    updated_at: chrono::Utc::now(),
                }])
            } else {
                Ok(vec![])
            }
        }
        async fn get_all_config(
            &self,
            _: &str,
        ) -> Result<Vec<crate::nexus::domain::entities::system::bot_config::BotGuildConfig>, crate::nexus::domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), crate::nexus::domain::errors::DomainError> {
            Ok(())
        }
        async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), crate::nexus::domain::errors::DomainError> {
            Ok(())
        }
    }

    let wheel = Arc::new(MockWheelRepo {
        claim_ok: true,
        logged: Mutex::new(vec![]),
        wallet_repo: Mutex::new(None),
    });
    let wallet = Arc::new(MockWalletRepo {
        initial_coins: 1000,
        saved: Mutex::new(vec![]),
    });
    let svc = PlayWheelService::new(
        wheel.clone(),
        wallet.clone(),
        Arc::new(MockConfigRepoWheelDisabled),
    );

    assert!(!svc.can_spin("g1", "u1").await.unwrap());
}


#[tokio::test]
async fn test_spin_cooldown_message_with_hours() {
    struct MockConfigRepoCooldownHours;
    #[async_trait::async_trait]
    impl crate::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository
        for MockConfigRepoCooldownHours
    {
        async fn get_definitions(
            &self,
        ) -> Result<
            Vec<crate::nexus::domain::entities::system::bot_config::BotDefinition>,
            crate::nexus::domain::errors::DomainError,
        > {
            Ok(vec![])
        }
        async fn get_config(
            &self,
            _guild_id: &str,
            bot_name: &str,
        ) -> Result<Vec<crate::nexus::domain::entities::system::bot_config::BotGuildConfig>, crate::nexus::domain::errors::DomainError> {
            if bot_name == "nexus-economy" {
                Ok(vec![
                    crate::nexus::domain::entities::system::bot_config::BotGuildConfig {
                        id: uuid::Uuid::nil(),
                        guild_id: "g1".into(),
                        bot_name: bot_name.to_string(),
                        config_key: "enabled".into(),
                        config_value: "true".into(),
                        updated_at: chrono::Utc::now(),
                    },
                    crate::nexus::domain::entities::system::bot_config::BotGuildConfig {
                        id: uuid::Uuid::nil(),
                        guild_id: "g1".into(),
                        bot_name: bot_name.to_string(),
                        config_key: "wheel_cooldown_hours".into(),
                        config_value: "6".into(),
                        updated_at: chrono::Utc::now(),
                    },
                ])
            } else {
                Ok(vec![])
            }
        }
        async fn get_all_config(
            &self,
            _: &str,
        ) -> Result<Vec<crate::nexus::domain::entities::system::bot_config::BotGuildConfig>, crate::nexus::domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), crate::nexus::domain::errors::DomainError> {
            Ok(())
        }
        async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), crate::nexus::domain::errors::DomainError> {
            Ok(())
        }
    }

    let wheel = Arc::new(MockWheelRepo {
        claim_ok: false,
        logged: Mutex::new(vec![]),
        wallet_repo: Mutex::new(None),
    });
    let wallet = Arc::new(MockWalletRepo {
        initial_coins: 1000,
        saved: Mutex::new(vec![]),
    });
    let svc_cooldown = PlayWheelService::new(
        wheel.clone(),
        wallet.clone(),
        Arc::new(MockConfigRepoCooldownHours),
    );

    let err = svc_cooldown.spin(cmd()).await.unwrap_err();
    // With 6 hours cooldown, the message should show "Reviens dans moins de 6 h."
    if let DomainError::Validation(msg) = err {
        assert!(msg.contains("6 h"));
    } else {
        panic!("Expected Validation error with cooldown message");
    }
}

#[tokio::test]
async fn test_spin_with_economy_disabled() {
    struct MockConfigRepoDisabled;
    #[async_trait::async_trait]
    impl crate::nexus::ports::outbound::system::bot_config_repository::BotConfigRepository
        for MockConfigRepoDisabled
    {
        async fn get_definitions(
            &self,
        ) -> Result<
            Vec<crate::nexus::domain::entities::system::bot_config::BotDefinition>,
            crate::nexus::domain::errors::DomainError,
        > {
            Ok(vec![])
        }
        async fn get_config(
            &self,
            _guild_id: &str,
            _bot_name: &str,
        ) -> Result<Vec<crate::nexus::domain::entities::system::bot_config::BotGuildConfig>, crate::nexus::domain::errors::DomainError> {
            // Return empty config, so enabled defaults to false (fail closed)
            Ok(vec![])
        }
        async fn get_all_config(
            &self,
            _: &str,
        ) -> Result<Vec<crate::nexus::domain::entities::system::bot_config::BotGuildConfig>, crate::nexus::domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), crate::nexus::domain::errors::DomainError> {
            Ok(())
        }
        async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), crate::nexus::domain::errors::DomainError> {
            Ok(())
        }
    }

    let wheel = Arc::new(MockWheelRepo {
        claim_ok: true,
        logged: Mutex::new(vec![]),
        wallet_repo: Mutex::new(None),
    });
    let wallet = Arc::new(MockWalletRepo {
        initial_coins: 1000,
        saved: Mutex::new(vec![]),
    });
    let svc = PlayWheelService::new(
        wheel.clone(),
        wallet.clone(),
        Arc::new(MockConfigRepoDisabled),
    );

    // Spin should fail because economy is disabled
    let err = svc.spin(cmd()).await.unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
}
