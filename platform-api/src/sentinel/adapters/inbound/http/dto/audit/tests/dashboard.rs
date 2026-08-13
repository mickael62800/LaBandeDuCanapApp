use super::*;
use chrono::TimeZone;
use chrono::Utc;
use platform_core::ops::domain::entities::log_entry::LogEntry;
use platform_core::sentinel::domain::entities::audit::dashboard_stats::DashboardStats;
use platform_core::sentinel::domain::entities::moderation::action::applied::ModerationAction;
use platform_core::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use platform_core::sentinel::domain::entities::moderation::infraction::Infraction;
use platform_core::sentinel::domain::entities::system::guild::Guild;
use platform_core::sentinel::domain::entities::system::rule::Rule;
use platform_core::sentinel::domain::enums::moderation::action::Action;
use platform_core::sentinel::domain::enums::moderation::flag_type::FlagType;
use platform_core::sentinel::domain::enums::moderation::moderation_gravity::ModerationGravity;
use uuid::Uuid;

fn flags() -> DetectionFlags {
    DetectionFlags {
        spam: false,
        insult: false,
        profanity: false,
        link: false,
        phishing: false,
    }
}

fn sample_infraction(action: Action, duration: Option<u64>) -> Infraction {
    Infraction {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "alice".into(),
        message_id: "m".into(),
        display_name: None,
        content: "hi".into(),
        flags: flags(),
        score: 0.5,
        action,
        reason: "reason".into(),
        duration,
        created_at: Utc.with_ymd_and_hms(2024, 5, 1, 10, 0, 0).unwrap(),
    }
}

fn sample_action() -> ModerationAction {
    ModerationAction {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "Mod".into(),
        target_id: "u".into(),
        target_name: "alice".into(),
        target_display_name: None,
        action_type: "ban".into(),
        reason: "r".into(),
        gravity: Some(ModerationGravity::High),
        duration: Some(7200),
        created_at: Utc::now(),
    }
}

fn sample_rule(flag_type: FlagType, warn: f64, delete: f64, mute: f64, ban: f64) -> Rule {
    sample_rule_pondere(flag_type, 2.5, warn, delete, mute, ban)
}

fn sample_rule_pondere(
    flag_type: FlagType,
    weight: f64,
    warn: f64,
    delete: f64,
    mute: f64,
    ban: f64,
) -> Rule {
    let now = Utc::now();
    Rule {
        id: Uuid::new_v4(),
        guild_id: "g42".into(),
        flag_type,
        weight,
        threshold_warn: warn,
        threshold_delete: delete,
        threshold_mute: mute,
        threshold_ban: ban,
        enabled: true,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn dashboard_stats_dto_preserves_all_fields() {
    // Le DTO compose deux domaines : le metier Sentinel et la sante des
    // services, qui vit desormais dans `ops-core`. Le test verifie justement
    // qu'aucun champ ne se perd a la jonction.
    let s = DashboardStats {
        total_servers: 10,
        total_users: 200,
        messages_today: 3000,
        infractions_today: 5,
        postgres_online: true,
    };
    let health = platform_core::ops::domain::entities::services_health::ServicesHealth {
        bots_online: 2,
        bots_total: 3,
        workers_online: 1,
        workers_total: 1,
        redis_online: false,
    };
    let dto = DashboardStatsDto::compose(s, health);
    assert_eq!(dto.total_servers, 10);
    assert_eq!(dto.messages_today, 3000);
    assert_eq!(dto.bots_online, 2);
    assert_eq!(dto.workers_total, 1);
    assert!(dto.postgres_online);
    assert!(!dto.redis_online);
}

#[test]
fn log_entry_dto_formats_timestamp_rfc3339() {
    let e = LogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap(),
        level: "INFO".into(),
        bot: "sentinel".into(),
        server: "g".into(),
        message: "msg".into(),
        category: "cat".into(),
        details: serde_json::json!({"k":"v"}),
    };
    let id = e.id.to_string();
    let dto = LogEntryDto::from(e);
    assert_eq!(dto.id, id);
    assert_eq!(dto.timestamp, "2024-01-02T03:04:05+00:00");
    assert_eq!(dto.details["k"], "v");
}

#[test]
fn create_log_dto_all_optional_except_message() {
    let dto: CreateLogDto = serde_json::from_value(serde_json::json!({"message": "hi"})).unwrap();
    assert_eq!(dto.message, "hi");
    assert!(dto.level.is_none());
    assert!(dto.bot.is_none());
    assert!(dto.details.is_none());
}

#[test]
fn dashboard_infraction_from_infraction_marks_automod_and_detection() {
    let inf = sample_infraction(Action::Warn, Some(600));
    let dto = DashboardInfractionDto::from(inf);
    assert_eq!(dto.moderator, "AutoMod");
    assert_eq!(dto.source, "detection");
    assert_eq!(dto.infraction_type, "warn");
    assert_eq!(dto.duration, Some(600));
    assert_eq!(dto.server, "g");
}

#[test]
fn dashboard_infraction_skips_none_duration_in_json() {
    let dto = DashboardInfractionDto::from(sample_infraction(Action::None, None));
    let v = serde_json::to_value(&dto).unwrap();
    assert!(v.get("duration").is_none());
}

#[test]
fn dashboard_infraction_from_action_marks_action_source() {
    let a = sample_action();
    let mod_name = a.moderator_name.clone();
    let dto = DashboardInfractionDto::from(a);
    assert_eq!(dto.source, "action");
    assert_eq!(dto.moderator, mod_name);
    assert_eq!(dto.infraction_type, "ban");
    assert_eq!(dto.duration, Some(7200));
}

// L'action affichee decrit ce que ce flag declenche SEUL : son poids
// compare a ses propres seuils. Elle ne dit pas quels seuils existent —
// c'etait le defaut de la version precedente, qui affichait « ban » pour
// toute regle puisque le seuil de bannissement vaut 9 par defaut.

#[test]
fn action_ban_quand_le_poids_atteint_le_seuil_de_ban() {
    let r = sample_rule_pondere(FlagType::Illicit, 9.0, 2.0, 4.0, 6.0, 9.0);
    assert_eq!(DashboardRuleDto::from(r).action, "ban");
}

#[test]
fn action_mute_quand_le_poids_atteint_le_seuil_de_mute() {
    let r = sample_rule_pondere(FlagType::Phishing, 7.0, 2.0, 4.0, 6.0, 9.0);
    assert_eq!(DashboardRuleDto::from(r).action, "mute");
}

#[test]
fn action_delete_quand_le_poids_atteint_le_seuil_de_suppression() {
    let r = sample_rule_pondere(FlagType::Insult, 5.0, 2.0, 4.0, 6.0, 9.0);
    assert_eq!(DashboardRuleDto::from(r).action, "delete");
}

#[test]
fn action_warn_quand_le_poids_atteint_le_seuil_d_avertissement() {
    let r = sample_rule_pondere(FlagType::Spam, 3.0, 2.0, 4.0, 6.0, 9.0);
    assert_eq!(DashboardRuleDto::from(r).action, "warn");
}

#[test]
fn action_none_quand_le_flag_ne_suffit_pas_seul() {
    // Le cas qui motivait la correction : un anti-spam de poids 1.5 sous un
    // seuil d'avertissement a 2 ne declenche RIEN seul. Il s'affichait
    // pourtant « BANNISSEMENT ».
    let r = sample_rule_pondere(FlagType::Spam, 1.5, 2.0, 4.0, 6.0, 9.0);
    assert_eq!(DashboardRuleDto::from(r).action, "none");
}

#[test]
fn action_ban_meme_si_les_seuils_intermediaires_sont_nuls() {
    // Seuils a zero : tout poids positif les franchit, y compris le ban.
    let r = sample_rule_pondere(FlagType::Nsfw, 0.5, 0.0, 0.0, 0.0, 0.0);
    assert_eq!(DashboardRuleDto::from(r).action, "ban");
}

#[test]
fn dashboard_rule_labels_known_flags() {
    for (flag, expected) in [
        (FlagType::Spam, "Anti-Spam"),
        (FlagType::Insult, "Anti-Insulte"),
        (FlagType::Link, "Anti-Lien"),
        (FlagType::Phishing, "Anti-Hameconnage"),
        (FlagType::Nsfw, "Anti-NSFW"),
        (FlagType::Illicit, "Anti-Illicite"),
        (FlagType::Anger, "Detection colere"),
        (FlagType::Rage, "Detection rage"),
        (FlagType::Threat, "Detection menace"),
        (FlagType::Harassment, "Detection harcelement"),
    ] {
        let r = sample_rule(flag.clone(), 0.0, 0.0, 0.0, 0.0);
        let dto = DashboardRuleDto::from(r);
        assert!(dto.name.starts_with(expected), "{:?} -> {}", flag, dto.name);
        assert!(dto.description.contains(expected));
    }
}

#[test]
fn dashboard_rule_description_includes_guild_and_weight() {
    let r = sample_rule(FlagType::Spam, 0.0, 0.0, 0.0, 0.0);
    let dto = DashboardRuleDto::from(r);
    assert!(dto.description.contains("g42"));
    assert!(dto.description.contains("2.5"));
}

#[test]
fn guild_dto_preserves_fields() {
    let g = Guild {
        guild_id: "g".into(),
        name: "Guild".into(),
        icon: Some("i".into()),
        member_count: 42,
        registered_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let dto = GuildDto::from(g);
    assert_eq!(dto.guild_id, "g".into());
    assert_eq!(dto.member_count, 42);
    assert_eq!(dto.icon.as_deref(), Some("i"));
}

#[test]
fn register_guild_dto_optional_fields() {
    let dto: RegisterGuildDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "name": "Guild"
    }))
    .unwrap();
    assert!(dto.icon.is_none());
    assert!(dto.member_count.is_none());
    assert!(dto.owner_id.is_none());
}

#[test]
fn guild_filter_params_empty_object() {
    let p: GuildFilterParams = serde_json::from_str("{}").unwrap();
    assert!(p.guild_id.is_none());
}
