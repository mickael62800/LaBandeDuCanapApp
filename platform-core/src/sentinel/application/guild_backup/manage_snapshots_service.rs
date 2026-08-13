//! Service application de gestion des sauvegardes de serveur.
//!
//! Logique PURE : validation legere + quota par guild, puis delegation au
//! repository. Aucune I/O directe (le repo est un port).

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::guild_backup::snapshot::GuildSnapshot;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::guild_backup::manage_snapshots::{
    ManageGuildSnapshotsUseCase, SnapshotId, SnapshotSummary,
};
use crate::sentinel::ports::outbound::guild_backup::snapshot_repository::SnapshotRepository;

/// Nombre maximum de sauvegardes conservees par guild. Au-dela, la plus
/// ancienne est evincee (rotation) pour laisser la place a la nouvelle.
pub const MAX_SNAPSHOTS_PER_GUILD: u32 = 20;

/// Longueur max d'un libelle (garde-fou contre les payloads abusifs).
const MAX_LABEL_LEN: usize = 200;

pub struct ManageGuildSnapshotsService {
    repo: Arc<dyn SnapshotRepository>,
}

impl ManageGuildSnapshotsService {
    pub fn new(repo: Arc<dyn SnapshotRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageGuildSnapshotsUseCase for ManageGuildSnapshotsService {
    async fn store_snapshot(&self, snapshot: GuildSnapshot) -> Result<SnapshotId, DomainError> {
        self.store_snapshot_with_quota(snapshot, MAX_SNAPSHOTS_PER_GUILD)
            .await
    }

    async fn store_snapshot_with_quota(
        &self,
        snapshot: GuildSnapshot,
        quota: u32,
    ) -> Result<SnapshotId, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(&snapshot.guild_id)?;
        let label = snapshot.meta.label.trim();
        crate::sentinel::application::validation::validate_non_empty(label, "label")?;
        if label.chars().count() > MAX_LABEL_LEN {
            return Err(DomainError::ValidationError(format!(
                "label trop long (max {MAX_LABEL_LEN} caracteres)"
            )));
        }

        // Quota borne a [1, 100] (garde-fou contre une config aberrante).
        let quota = quota.clamp(1, 100);

        // Rotation : si le quota est atteint, on evince la/les plus anciennes
        // pour garder au plus `quota - 1` avant l'insertion.
        while self.repo.count(&snapshot.guild_id).await? >= quota {
            match self.repo.oldest_id(&snapshot.guild_id).await? {
                Some(id) => {
                    self.repo.delete(id).await?;
                }
                // Incoherence (count > 0 mais pas d'oldest) : on stoppe pour
                // ne pas boucler indefiniment.
                None => break,
            }
        }

        self.repo.insert(&snapshot).await
    }

    async fn list_snapshots(&self, guild_id: &str) -> Result<Vec<SnapshotSummary>, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(guild_id)?;
        self.repo.list(guild_id).await
    }

    async fn get_snapshot(&self, snapshot_id: SnapshotId) -> Result<GuildSnapshot, DomainError> {
        self.repo
            .get(snapshot_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Sauvegarde {snapshot_id}")))
    }

    async fn delete_snapshot(&self, snapshot_id: SnapshotId) -> Result<bool, DomainError> {
        self.repo.delete(snapshot_id).await
    }

    async fn rename_snapshot(
        &self,
        snapshot_id: SnapshotId,
        label: &str,
    ) -> Result<bool, DomainError> {
        let label = label.trim();
        crate::sentinel::application::validation::validate_non_empty(label, "label")?;
        if label.chars().count() > MAX_LABEL_LEN {
            return Err(DomainError::ValidationError(format!(
                "label trop long (max {MAX_LABEL_LEN} caracteres)"
            )));
        }
        self.repo.rename(snapshot_id, label).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sentinel::domain::entities::guild_backup::snapshot::{
        GuildSettings, GuildSnapshot, SnapshotMeta, SCHEMA_VERSION,
    };
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    /// Repo en memoire pour tester la logique pure du service.
    #[derive(Default)]
    struct InMemoryRepo {
        // (id, snapshot) ordonne par insertion (le plus ancien en tete).
        rows: Mutex<Vec<(Uuid, GuildSnapshot)>>,
    }

    #[async_trait]
    impl SnapshotRepository for InMemoryRepo {
        async fn insert(&self, snapshot: &GuildSnapshot) -> Result<Uuid, DomainError> {
            let id = Uuid::new_v4();
            self.rows.lock().unwrap().push((id, snapshot.clone()));
            Ok(id)
        }
        async fn list(&self, guild_id: &str) -> Result<Vec<SnapshotSummary>, DomainError> {
            let rows = self.rows.lock().unwrap();
            Ok(rows
                .iter()
                .rev()
                .filter(|(_, s)| s.guild_id == guild_id)
                .map(|(id, s)| SnapshotSummary {
                    id: *id,
                    guild_id: s.guild_id.clone(),
                    label: s.meta.label.clone(),
                    created_at: s.meta.created_at.clone(),
                    created_by: s.meta.created_by.clone(),
                    schema_version: s.meta.schema_version,
                    role_count: s.role_count() as u32,
                    channel_count: s.channel_count() as u32,
                })
                .collect())
        }
        async fn get(&self, id: Uuid) -> Result<Option<GuildSnapshot>, DomainError> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .find(|(rid, _)| *rid == id)
                .map(|(_, s)| s.clone()))
        }
        async fn delete(&self, id: Uuid) -> Result<bool, DomainError> {
            let mut rows = self.rows.lock().unwrap();
            let before = rows.len();
            rows.retain(|(rid, _)| *rid != id);
            Ok(rows.len() != before)
        }
        async fn count(&self, guild_id: &str) -> Result<u32, DomainError> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, s)| s.guild_id == guild_id)
                .count() as u32)
        }
        async fn rename(&self, id: Uuid, label: &str) -> Result<bool, DomainError> {
            let mut rows = self.rows.lock().unwrap();
            match rows.iter_mut().find(|(rid, _)| *rid == id) {
                Some((_, s)) => {
                    s.meta.label = label.to_string();
                    Ok(true)
                }
                None => Ok(false),
            }
        }
        async fn oldest_id(&self, guild_id: &str) -> Result<Option<Uuid>, DomainError> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .find(|(_, s)| s.guild_id == guild_id)
                .map(|(id, _)| *id))
        }
    }

    fn snapshot(guild_id: &str, label: &str) -> GuildSnapshot {
        GuildSnapshot {
            guild_id: guild_id.to_string(),
            meta: SnapshotMeta {
                label: label.to_string(),
                created_at: "2026-07-07T00:00:00Z".to_string(),
                created_by: Some("42".to_string()),
                schema_version: SCHEMA_VERSION,
            },
            settings: GuildSettings {
                name: "Serveur".to_string(),
                icon: None,
                verification_level: 1,
                default_notifications: 0,
                explicit_content_filter: 0,
                afk_channel_old_id: None,
                afk_timeout: 300,
                system_channel_old_id: None,
                everyone_permissions: String::new(),
            },
            roles: vec![],
            categories: vec![],
            channels: vec![],
            bans: vec![],
            emojis: vec![],
            member_roles: BTreeMap::new(),
        }
    }

    fn service() -> (ManageGuildSnapshotsService, Arc<InMemoryRepo>) {
        let repo = Arc::new(InMemoryRepo::default());
        (ManageGuildSnapshotsService::new(repo.clone()), repo)
    }

    #[tokio::test]
    async fn store_then_get_roundtrip() {
        let (svc, _) = service();
        let id = svc.store_snapshot(snapshot("g1", "backup")).await.unwrap();
        let loaded = svc.get_snapshot(id).await.unwrap();
        assert_eq!(loaded.guild_id, "g1");
        assert_eq!(loaded.meta.label, "backup");
    }

    #[tokio::test]
    async fn store_rejects_empty_label() {
        let (svc, _) = service();
        let err = svc.store_snapshot(snapshot("g1", "   ")).await.unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }

    #[tokio::test]
    async fn store_rejects_empty_guild() {
        let (svc, _) = service();
        let err = svc.store_snapshot(snapshot("", "ok")).await.unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }

    #[tokio::test]
    async fn get_missing_is_not_found() {
        let (svc, _) = service();
        let err = svc.get_snapshot(Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn delete_reports_existence() {
        let (svc, _) = service();
        let id = svc.store_snapshot(snapshot("g1", "a")).await.unwrap();
        assert!(svc.delete_snapshot(id).await.unwrap());
        assert!(!svc.delete_snapshot(id).await.unwrap());
    }

    #[tokio::test]
    async fn rename_updates_label() {
        let (svc, _) = service();
        let id = svc.store_snapshot(snapshot("g1", "old")).await.unwrap();
        assert!(svc.rename_snapshot(id, "new").await.unwrap());
        assert_eq!(svc.get_snapshot(id).await.unwrap().meta.label, "new");
    }

    #[tokio::test]
    async fn rename_rejects_empty_label() {
        let (svc, _) = service();
        let id = svc.store_snapshot(snapshot("g1", "old")).await.unwrap();
        let err = svc.rename_snapshot(id, "  ").await.unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }

    #[tokio::test]
    async fn rename_missing_returns_false() {
        let (svc, _) = service();
        assert!(!svc.rename_snapshot(Uuid::new_v4(), "x").await.unwrap());
    }

    #[tokio::test]
    async fn quota_evicts_oldest() {
        let (svc, repo) = service();
        for i in 0..MAX_SNAPSHOTS_PER_GUILD {
            svc.store_snapshot(snapshot("g1", &format!("b{i}")))
                .await
                .unwrap();
        }
        assert_eq!(repo.count("g1").await.unwrap(), MAX_SNAPSHOTS_PER_GUILD);
        // La N+1e capture evince la plus ancienne : le total reste borne.
        svc.store_snapshot(snapshot("g1", "newest")).await.unwrap();
        assert_eq!(repo.count("g1").await.unwrap(), MAX_SNAPSHOTS_PER_GUILD);
        // La plus ancienne ("b0") a disparu.
        let labels: Vec<String> = svc
            .list_snapshots("g1")
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.label)
            .collect();
        assert!(!labels.contains(&"b0".to_string()));
        assert!(labels.contains(&"newest".to_string()));
    }

    #[tokio::test]
    async fn custom_quota_evicts_at_configured_limit() {
        let (svc, repo) = service();
        // Quota configure a 3 : au-dela, la plus ancienne est evincee.
        for i in 0..3 {
            svc.store_snapshot_with_quota(snapshot("g1", &format!("b{i}")), 3)
                .await
                .unwrap();
        }
        assert_eq!(repo.count("g1").await.unwrap(), 3);
        svc.store_snapshot_with_quota(snapshot("g1", "newest"), 3)
            .await
            .unwrap();
        assert_eq!(repo.count("g1").await.unwrap(), 3);
        let labels: Vec<String> = svc
            .list_snapshots("g1")
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.label)
            .collect();
        assert!(!labels.contains(&"b0".to_string()));
        assert!(labels.contains(&"newest".to_string()));
    }

    #[tokio::test]
    async fn quota_zero_is_clamped_to_one() {
        let (svc, repo) = service();
        svc.store_snapshot_with_quota(snapshot("g1", "a"), 0)
            .await
            .unwrap();
        svc.store_snapshot_with_quota(snapshot("g1", "b"), 0)
            .await
            .unwrap();
        // quota 0 borne a 1 : une seule sauvegarde conservee.
        assert_eq!(repo.count("g1").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn quota_is_per_guild() {
        let (svc, repo) = service();
        for i in 0..MAX_SNAPSHOTS_PER_GUILD {
            svc.store_snapshot(snapshot("g1", &format!("b{i}")))
                .await
                .unwrap();
        }
        svc.store_snapshot(snapshot("g2", "other")).await.unwrap();
        // g2 n'est pas affecte par le quota de g1.
        assert_eq!(repo.count("g2").await.unwrap(), 1);
    }
}
