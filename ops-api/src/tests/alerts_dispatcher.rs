use super::*;

fn rule(comparator: &str, threshold: Option<f64>) -> AlertRule {
    AlertRule {
        id: "t".into(),
        label: "T".into(),
        metric: "cpu_percent".into(),
        comparator: comparator.into(),
        threshold,
        severity: "warning".into(),
        cooldown_secs: 60,
    }
}

#[test]
fn triggers_gt_above_threshold() {
    let r = rule("gt", Some(90.0));
    assert!(r.triggers(95.0));
    assert!(!r.triggers(90.0)); // strict
    assert!(!r.triggers(80.0));
}

#[test]
fn triggers_lt_below_threshold() {
    let r = rule("lt", Some(14.0));
    assert!(r.triggers(10.0));
    assert!(!r.triggers(14.0));
    assert!(!r.triggers(20.0));
}

#[test]
fn triggers_false_without_threshold() {
    // Les metriques booleennes (service_offline...) n'ont pas de seuil numerique.
    assert!(!rule("gt", None).triggers(999.0));
}

#[test]
fn triggers_false_unknown_comparator() {
    assert!(!rule("eq", Some(1.0)).triggers(1.0));
}

#[test]
fn color_by_severity() {
    let mut r = rule("gt", Some(1.0));
    r.severity = "critical".into();
    assert_eq!(r.color(), 0xE74C3C);
    r.severity = "info".into();
    assert_eq!(r.color(), 0x3498DB);
    r.severity = "warning".into();
    assert_eq!(r.color(), 0xF39C12);
}

#[test]
fn evaluate_service_offline_one_alert_per_service() {
    let mut r = rule("gt", None);
    r.metric = "service_offline".into();
    r.label = "Offline".into();
    let m = Metrics {
        cpu_percent: 0.0,
        mem_percent: 0.0,
        disk_percent: 0.0,
        auth_failures_1h: 0.0,
        tls_expiry_days: None,
        offline_services: vec!["automod-bot".into(), "audit-worker".into()],
        container_changes: vec![],
    };
    let alerts = evaluate(&r, &m);
    assert_eq!(alerts.len(), 2);
    // Cle de dedup = nom du service (une alerte distincte par service).
    assert_eq!(alerts[0].0, "automod-bot");
    assert_eq!(alerts[1].0, "audit-worker");
}

#[test]
fn evaluate_cpu_below_threshold_no_alert() {
    let r = rule("gt", Some(90.0));
    let m = Metrics {
        cpu_percent: 50.0,
        mem_percent: 0.0,
        disk_percent: 0.0,
        auth_failures_1h: 0.0,
        tls_expiry_days: None,
        offline_services: vec![],
        container_changes: vec![],
    };
    assert!(evaluate(&r, &m).is_empty());
}
