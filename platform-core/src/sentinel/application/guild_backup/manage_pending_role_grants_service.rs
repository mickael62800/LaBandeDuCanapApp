//! Service application des re-attributions de roles en attente.
//!
//! Logique PURE : validation legere + normalisation (drop des grants vides /
//! guild_id/user_id vides) puis delegation au repository. Aucune I/O directe.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::guild_backup::pending_role_grant::PendingRoleGrant;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::guild_backup::manage_pending_role_grants::ManagePendingRoleGrantsUseCase;
use crate::sentinel::ports::outbound::guild_backup::pending_role_grant_repository::PendingRoleGrantRepository;

pub struct ManagePendingRoleGrantsService {
    repo: Arc<dyn PendingRoleGrantRepository>,
}

impl ManagePendingRoleGrantsService {
    pub fn new(repo: Arc<dyn PendingRoleGrantRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManagePendingRoleGrantsUseCase for ManagePendingRoleGrantsService {
    async fn save_grants(
        &self,
        guild_id: &str,
        grants: Vec<PendingRoleGrant>,
    ) -> Result<u64, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(guild_id)?;
        // Normalisation : force le guild_id autoritaire, ecarte les entrees sans
        // user_id ou sans roles (rien a re-attribuer).
        let cleaned: Vec<PendingRoleGrant> = grants
            .into_iter()
            .filter(|g| !g.user_id.trim().is_empty() && !g.role_ids.is_empty())
            .map(|g| PendingRoleGrant {
                guild_id: guild_id.to_string(),
                user_id: g.user_id,
                role_ids: g.role_ids,
            })
            .collect();
        if cleaned.is_empty() {
            return Ok(0);
        }
        self.repo.upsert_many(&cleaned).await
    }

    async fn take_grant(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Vec<String>>, DomainError> {
        if guild_id.trim().is_empty() || user_id.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "guild_id et user_id requis".into(),
            ));
        }
        self.repo.take(guild_id, user_id).await
    }

    async fn clear_guild(&self, guild_id: &str) -> Result<u64, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(guild_id)?;
        self.repo.clear_guild(guild_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Repo en memoire : `take` retire l'entree (simule le DELETE ... RETURNING).
    #[derive(Default)]
    struct InMemoryRepo {
        // clef (guild_id, user_id) -> role_ids
        rows: Mutex<HashMap<(String, String), Vec<String>>>,
    }

    #[async_trait]
    impl PendingRoleGrantRepository for InMemoryRepo {
        async fn upsert_many(&self, grants: &[PendingRoleGrant]) -> Result<u64, DomainError> {
            let mut rows = self.rows.lock().unwrap();
            for g in grants {
                rows.insert((g.guild_id.clone(), g.user_id.clone()), g.role_ids.clone());
            }
            Ok(grants.len() as u64)
        }
        async fn take(
            &self,
            guild_id: &str,
            user_id: &str,
        ) -> Result<Option<Vec<String>>, DomainError> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .remove(&(guild_id.to_string(), user_id.to_string())))
        }
        async fn clear_guild(&self, guild_id: &str) -> Result<u64, DomainError> {
            let mut rows = self.rows.lock().unwrap();
            let before = rows.len();
            rows.retain(|(g, _), _| g != guild_id);
            Ok((before - rows.len()) as u64)
        }
    }

    fn service() -> (ManagePendingRoleGrantsService, Arc<InMemoryRepo>) {
        let repo = Arc::new(InMemoryRepo::default());
        (ManagePendingRoleGrantsService::new(repo.clone()), repo)
    }

    fn grant(user: &str, roles: &[&str]) -> PendingRoleGrant {
        PendingRoleGrant {
            guild_id: String::new(),
            user_id: user.to_string(),
            role_ids: roles.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[tokio::test]
    async fn save_then_take_roundtrip() {
        let (svc, _) = service();
        let n = svc
            .save_grants("g1", vec![grant("u1", &["r1", "r2"])])
            .await
            .unwrap();
        assert_eq!(n, 1);
        let taken = svc.take_grant("g1", "u1").await.unwrap();
        assert_eq!(taken, Some(vec!["r1".to_string(), "r2".to_string()]));
    }

    #[tokio::test]
    async fn take_is_idempotent_single_shot() {
        let (svc, _) = service();
        svc.save_grants("g1", vec![grant("u1", &["r1"])])
            .await
            .unwrap();
        // Premier take rend les roles, le second ne rend plus rien (supprime).
        assert!(svc.take_grant("g1", "u1").await.unwrap().is_some());
        assert!(svc.take_grant("g1", "u1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn take_missing_is_none() {
        let (svc, _) = service();
        assert!(svc.take_grant("g1", "ghost").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn save_drops_empty_grants() {
        let (svc, repo) = service();
        let n = svc
            .save_grants(
                "g1",
                vec![grant("u1", &[]), grant("", &["r1"]), grant("u2", &["r3"])],
            )
            .await
            .unwrap();
        // Seul (u2 -> r3) est retenu.
        assert_eq!(n, 1);
        assert_eq!(repo.rows.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn save_forces_authoritative_guild_id() {
        let (svc, _) = service();
        // Le guild_id du grant est ignore au profit de celui passe en argument.
        let mut g = grant("u1", &["r1"]);
        g.guild_id = "WRONG".to_string();
        svc.save_grants("g1", vec![g]).await.unwrap();
        assert!(svc.take_grant("g1", "u1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn upsert_replaces_previous() {
        let (svc, _) = service();
        svc.save_grants("g1", vec![grant("u1", &["r1"])])
            .await
            .unwrap();
        svc.save_grants("g1", vec![grant("u1", &["r2", "r3"])])
            .await
            .unwrap();
        assert_eq!(
            svc.take_grant("g1", "u1").await.unwrap(),
            Some(vec!["r2".to_string(), "r3".to_string()])
        );
    }

    #[tokio::test]
    async fn clear_guild_purges_only_target() {
        let (svc, _) = service();
        svc.save_grants("g1", vec![grant("u1", &["r1"]), grant("u2", &["r2"])])
            .await
            .unwrap();
        svc.save_grants("g2", vec![grant("u3", &["r3"])])
            .await
            .unwrap();
        assert_eq!(svc.clear_guild("g1").await.unwrap(), 2);
        assert!(svc.take_grant("g1", "u1").await.unwrap().is_none());
        // g2 intact.
        assert!(svc.take_grant("g2", "u3").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn empty_guild_id_is_rejected() {
        let (svc, _) = service();
        assert!(matches!(
            svc.save_grants("", vec![grant("u1", &["r1"])])
                .await
                .unwrap_err(),
            DomainError::ValidationError(_)
        ));
        assert!(matches!(
            svc.take_grant("", "u1").await.unwrap_err(),
            DomainError::ValidationError(_)
        ));
        assert!(matches!(
            svc.clear_guild("").await.unwrap_err(),
            DomainError::ValidationError(_)
        ));
    }
}
