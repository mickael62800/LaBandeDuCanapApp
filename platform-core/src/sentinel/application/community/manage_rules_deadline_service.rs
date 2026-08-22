//! Use case du delai d'acceptation du reglement : lit le reglage de la guilde,
//! calcule l'echeance et delegue la persistance au repo.
//!
//! Toute la regle metier vit ici ou dans le domaine ; le SQL dans
//! `RulesDeadlineRepository`, le handler HTTP ne fait que parser et mapper.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::community::rules_deadline::RulesDeadlineSettings;
use crate::sentinel::domain::entities::system::bot_names::WELCOME_BOT;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_rules_deadline::ManageRulesDeadlineUseCase;
use crate::sentinel::ports::outbound::community::rules_deadline_repository::RulesDeadlineRepository;
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub struct ManageRulesDeadlineService {
    repo: Arc<dyn RulesDeadlineRepository>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ManageRulesDeadlineService {
    pub fn new(
        repo: Arc<dyn RulesDeadlineRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            repo,
            bot_config_repo,
        }
    }
}

fn parse_bool(valeur: Option<&str>, defaut: bool) -> bool {
    match valeur.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
        Some("true") | Some("1") | Some("yes") | Some("on") => true,
        Some("false") | Some("0") | Some("no") | Some("off") => false,
        _ => defaut,
    }
}

#[async_trait]
impl ManageRulesDeadlineUseCase for ManageRulesDeadlineService {
    async fn settings(&self, guild_id: &str) -> Result<RulesDeadlineSettings, DomainError> {
        // Config illisible : on retombe sur les defauts. Or le defaut est
        // `enabled = false`, donc l'accueil continue sans compte a rebours —
        // c'est le repli sur : rien ne se passe, personne n'est expulse.
        let configs = self
            .bot_config_repo
            .get_config(guild_id, WELCOME_BOT)
            .await
            .unwrap_or_default();

        let brut = |cle: &str| {
            configs
                .iter()
                .find(|c| c.config_key == cle)
                .map(|c| c.config_value.as_str())
        };
        let defauts = RulesDeadlineSettings::default();

        Ok(RulesDeadlineSettings {
            enabled: parse_bool(brut("rules_deadline_enabled"), false),
            deadline_secs: brut("rules_deadline_secs")
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(defauts.deadline_secs),
            reminder_secs: brut("rules_reminder_secs")
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(defauts.reminder_secs),
            kick_enabled: parse_bool(brut("rules_kick_enabled"), defauts.kick_enabled),
        }
        .sanitized())
    }

    async fn start(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<RulesDeadlineSettings, DomainError> {
        let reglages = self.settings(guild_id).await?;
        // Fail closed : tant que la guilde n'a pas active le delai, aucune
        // echeance n'est posee. Sans cette porte, activer le reglage plus tard
        // expulserait d'un coup toute une file constituee a son insu.
        if reglages.enabled {
            let expires_at = reglages.expires_from(chrono::Utc::now());
            self.repo
                .insert_if_absent(guild_id, user_id, expires_at)
                .await?;
        }
        Ok(reglages)
    }

    async fn clear(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        // Inconditionnel, meme si le delai est desactive : une echeance posee
        // avant la desactivation doit pouvoir etre levee.
        self.repo.delete(guild_id, user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sentinel::domain::entities::community::rules_deadline::PendingRulesDeadline;
    use crate::sentinel::domain::entities::system::bot_config::{BotDefinition, BotGuildConfig};
    use crate::sentinel::domain::entities::system::discord_ids::GuildId;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    #[derive(Default)]
    struct RepoEspion {
        poses: Mutex<Vec<(String, String, DateTime<Utc>)>>,
        effaces: Mutex<Vec<(String, String)>>,
    }

    #[async_trait]
    impl RulesDeadlineRepository for RepoEspion {
        async fn insert_if_absent(
            &self,
            guild_id: &str,
            user_id: &str,
            expires_at: DateTime<Utc>,
        ) -> Result<(), DomainError> {
            self.poses
                .lock()
                .unwrap()
                .push((guild_id.into(), user_id.into(), expires_at));
            Ok(())
        }
        async fn list_reminder_due(
            &self,
            _limit: i64,
        ) -> Result<Vec<PendingRulesDeadline>, DomainError> {
            Ok(vec![])
        }
        async fn claim_reminder(&self, _g: &str, _u: &str) -> Result<bool, DomainError> {
            Ok(true)
        }
        async fn list_expired(
            &self,
            _limit: i64,
        ) -> Result<Vec<PendingRulesDeadline>, DomainError> {
            Ok(vec![])
        }
        async fn delete(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
            self.effaces
                .lock()
                .unwrap()
                .push((guild_id.into(), user_id.into()));
            Ok(())
        }
    }

    struct ConfigStub(Vec<(&'static str, &'static str)>);

    #[async_trait]
    impl BotConfigRepository for ConfigStub {
        async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
            Ok(vec![])
        }
        async fn get_config(
            &self,
            _guild_id: &str,
            _bot_name: &str,
        ) -> Result<Vec<BotGuildConfig>, DomainError> {
            Ok(self
                .0
                .iter()
                .map(|(k, v)| BotGuildConfig {
                    id: uuid::Uuid::nil(),
                    guild_id: GuildId::new("1"),
                    bot_name: WELCOME_BOT.into(),
                    config_key: (*k).into(),
                    config_value: (*v).into(),
                    updated_at: Utc::now(),
                })
                .collect())
        }
        async fn get_all_config(&self, _g: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
            Ok(vec![])
        }
        async fn set_config(
            &self,
            _g: &str,
            _b: &str,
            _k: &str,
            _v: &str,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn delete_config(&self, _g: &str, _b: &str, _k: &str) -> Result<(), DomainError> {
            Ok(())
        }
    }

    fn service(
        config: Vec<(&'static str, &'static str)>,
    ) -> (ManageRulesDeadlineService, Arc<RepoEspion>) {
        let repo = Arc::new(RepoEspion::default());
        let service = ManageRulesDeadlineService::new(repo.clone(), Arc::new(ConfigStub(config)));
        (service, repo)
    }

    #[tokio::test]
    async fn sans_activation_aucune_echeance_n_est_posee() {
        // Le piege que cette porte ferme : activer le reglage plus tard
        // expulserait d'un coup toute une file constituee a l'insu du serveur.
        let (s, repo) = service(vec![]);
        let applique = s.start("g1", "u1").await.unwrap();
        assert!(!applique.enabled);
        assert!(repo.poses.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn une_fois_active_l_echeance_suit_le_reglage_de_la_guilde() {
        let (s, repo) = service(vec![
            ("rules_deadline_enabled", "true"),
            ("rules_deadline_secs", "7200"), // 2 h
        ]);
        let avant = chrono::Utc::now();
        let applique = s.start("g1", "u1").await.unwrap();
        assert!(applique.enabled);
        assert_eq!(applique.deadline_secs, 7200);

        let poses = repo.poses.lock().unwrap();
        assert_eq!(poses.len(), 1);
        let ecart = (poses[0].2 - avant).num_seconds();
        assert!((7195..=7205).contains(&ecart), "echeance a {ecart} s");
    }

    #[tokio::test]
    async fn un_reglage_aberrant_est_borne_au_lieu_de_faire_echouer() {
        // Un delai a zero expulserait tout le monde des l'arrivee.
        let (s, _) = service(vec![
            ("rules_deadline_enabled", "true"),
            ("rules_deadline_secs", "0"),
        ]);
        let applique = s.start("g1", "u1").await.unwrap();
        assert_eq!(
            applique.deadline_secs,
            crate::sentinel::domain::entities::community::rules_deadline::MIN_DEADLINE_SECS
        );
    }

    #[tokio::test]
    async fn une_config_illisible_ne_pose_rien() {
        let (s, repo) = service(vec![("rules_deadline_enabled", "peut-etre")]);
        let applique = s.start("g1", "u1").await.unwrap();
        assert!(!applique.enabled);
        assert!(repo.poses.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn l_echeance_se_leve_meme_quand_le_delai_est_desactive() {
        // Une echeance posee avant la desactivation doit pouvoir etre levee,
        // sinon elle resterait en base sans que rien ne puisse la retirer.
        let (s, repo) = service(vec![]);
        s.clear("g1", "u1").await.unwrap();
        assert_eq!(repo.effaces.lock().unwrap().len(), 1);
    }
}
