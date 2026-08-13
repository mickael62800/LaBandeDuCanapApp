use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::moderation_anomaly::AnomalyThresholds;
use crate::sentinel::ports::inbound::audit::detect_moderation_anomaly::DetectAnomalyCommand;
use crate::sentinel::ports::inbound::audit::detect_moderation_anomaly::DetectModerationAnomalyUseCase;
use crate::sentinel::ports::outbound::audit::moderation_anomaly_counter::ModerationAnomalyCounter;

use super::super::detect_moderation_anomaly_service::DetectModerationAnomalyService;

/// Compteur fake : simple compteur monotone par (guild, categorie), remis a
/// zero par `reset`. Suffisant pour tester la DECISION (la fenetre temporelle
/// est testee cote adapter serveur).
#[derive(Default)]
struct FakeCounter {
    counts: Mutex<HashMap<(String, String), usize>>,
}

#[async_trait]
impl ModerationAnomalyCounter for FakeCounter {
    async fn record(&self, guild_id: &str, category: &str, _window_secs: u64) -> usize {
        let mut counts = self.counts.lock().unwrap();
        let entry = counts
            .entry((guild_id.to_string(), category.to_string()))
            .or_insert(0);
        *entry += 1;
        *entry
    }

    async fn reset(&self, guild_id: &str, category: &str) {
        self.counts
            .lock()
            .unwrap()
            .remove(&(guild_id.to_string(), category.to_string()));
    }
}

fn thresholds() -> AnomalyThresholds {
    AnomalyThresholds {
        mass_ban: 3,
        mass_delete: 5,
        mass_role_change: 4,
    }
}

fn service() -> DetectModerationAnomalyService {
    DetectModerationAnomalyService::new(Arc::new(FakeCounter::default()))
}

fn cmd(guild: &str, category: &str) -> DetectAnomalyCommand {
    DetectAnomalyCommand {
        guild_id: guild.to_string(),
        category: category.to_string(),
        increment: 1,
        window_secs: 60,
        thresholds: thresholds(),
    }
}

#[tokio::test]
async fn no_alert_below_threshold() {
    let svc = service();
    assert!(svc.detect(cmd("1", "ban")).await.is_none());
    assert!(svc.detect(cmd("1", "ban")).await.is_none());
}

#[tokio::test]
async fn alert_at_threshold() {
    let svc = service();
    assert!(svc.detect(cmd("1", "ban")).await.is_none());
    assert!(svc.detect(cmd("1", "ban")).await.is_none());
    let alert = svc.detect(cmd("1", "ban")).await.expect("alerte attendue");
    assert_eq!(alert.anomaly_type, "mass_ban");
    assert_eq!(alert.count, 3);
    assert_eq!(alert.window_secs, 60);
}

#[tokio::test]
async fn reset_after_alert() {
    let svc = service();
    svc.detect(cmd("1", "ban")).await;
    svc.detect(cmd("1", "ban")).await;
    assert!(svc.detect(cmd("1", "ban")).await.is_some());
    // Apres reset, il faut de nouveau atteindre le seuil.
    assert!(svc.detect(cmd("1", "ban")).await.is_none());
    assert!(svc.detect(cmd("1", "ban")).await.is_none());
    assert!(svc.detect(cmd("1", "ban")).await.is_some());
}

#[tokio::test]
async fn different_guilds_independent() {
    let svc = service();
    svc.detect(cmd("1", "ban")).await;
    svc.detect(cmd("1", "ban")).await;
    svc.detect(cmd("2", "ban")).await;
    assert!(svc.detect(cmd("2", "ban")).await.is_none()); // guild 2 = 2
    assert!(svc.detect(cmd("1", "ban")).await.is_some()); // guild 1 = 3 -> alerte
}

#[tokio::test]
async fn delete_threshold_different() {
    let svc = service();
    for _ in 0..4 {
        assert!(svc.detect(cmd("1", "delete")).await.is_none());
    }
    assert!(svc.detect(cmd("1", "delete")).await.is_some());
}

#[tokio::test]
async fn role_change_threshold() {
    let svc = service();
    for _ in 0..3 {
        assert!(svc.detect(cmd("1", "role_change")).await.is_none());
    }
    assert!(svc.detect(cmd("1", "role_change")).await.is_some());
}

#[tokio::test]
async fn kick_uses_ban_threshold() {
    let svc = service();
    svc.detect(cmd("1", "kick")).await;
    svc.detect(cmd("1", "kick")).await;
    assert!(svc.detect(cmd("1", "kick")).await.is_some());
}

#[tokio::test]
async fn unknown_category_never_alerts() {
    let svc = service();
    for _ in 0..100 {
        assert!(svc.detect(cmd("1", "unknown")).await.is_none());
    }
}

#[tokio::test]
async fn bulk_increment_alerts_once() {
    let svc = service();
    // 10 deletes d'un coup, seuil delete = 5 -> une alerte au franchissement.
    let bulk = DetectAnomalyCommand {
        guild_id: "1".to_string(),
        category: "delete".to_string(),
        increment: 10,
        window_secs: 60,
        thresholds: thresholds(),
    };
    let alert = svc.detect(bulk).await.expect("alerte attendue");
    assert_eq!(alert.anomaly_type, "mass_delete");
    assert_eq!(alert.count, 5);
}
