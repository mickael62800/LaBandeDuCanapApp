use super::*;
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

struct MockBotConfigRepo;

#[async_trait]
impl BotConfigRepository for MockBotConfigRepo {
    async fn get_definitions(
        &self,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::system::bot_config::BotDefinition>,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(vec![])
    }
    async fn get_config(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::system::bot_config::BotGuildConfig>,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(vec![])
    }
    async fn get_all_config(
        &self,
        _: &str,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::system::bot_config::BotGuildConfig>,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(vec![])
    }
    async fn set_config(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), crate::sentinel::domain::errors::DomainError> {
        Ok(())
    }
    async fn delete_config(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), crate::sentinel::domain::errors::DomainError> {
        Ok(())
    }
}

struct MockRuleRepo;

#[async_trait]
impl RuleRepository for MockRuleRepo {
    async fn find_by_guild(
        &self,
        _: &str,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::system::rule::Rule>,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(vec![])
    }
    async fn find_all(
        &self,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::system::rule::Rule>,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(vec![])
    }
    async fn find_by_id(
        &self,
        _: uuid::Uuid,
    ) -> Result<
        Option<crate::sentinel::domain::entities::system::rule::Rule>,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(None)
    }
    async fn save(
        &self,
        rule: &crate::sentinel::domain::entities::system::rule::Rule,
    ) -> Result<
        crate::sentinel::domain::entities::system::rule::Rule,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(rule.clone())
    }
    async fn toggle(
        &self,
        _: uuid::Uuid,
        _: bool,
    ) -> Result<(), crate::sentinel::domain::errors::DomainError> {
        Ok(())
    }
    async fn delete(
        &self,
        _: uuid::Uuid,
    ) -> Result<(), crate::sentinel::domain::errors::DomainError> {
        Ok(())
    }
    async fn seed_defaults(
        &self,
        _: &[crate::sentinel::domain::entities::system::rule::Rule],
    ) -> Result<(), crate::sentinel::domain::errors::DomainError> {
        Ok(())
    }
}

struct MockInfractionRepo;

#[async_trait]
impl InfractionRepository for MockInfractionRepo {
    async fn count_by_action_for_user(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Vec<(String, u64)>, crate::sentinel::domain::errors::DomainError> {
        Ok(vec![])
    }
    async fn save(
        &self,
        _: &crate::sentinel::domain::entities::moderation::infraction::Infraction,
    ) -> Result<(), crate::sentinel::domain::errors::DomainError> {
        Ok(())
    }
    async fn find_by_guild(
        &self,
        _: &str,
        _: &crate::sentinel::ports::inbound::moderation::manage_infractions::InfractionFilters,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::moderation::infraction::Infraction>,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(vec![])
    }
    async fn find_all(
        &self,
        _: i64,
        _: i64,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::moderation::infraction::Infraction>,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(vec![])
    }
    async fn count_today(&self) -> Result<u64, crate::sentinel::domain::errors::DomainError> {
        Ok(0)
    }
    async fn find_by_id(
        &self,
        _: &str,
    ) -> Result<
        Option<crate::sentinel::domain::entities::moderation::infraction::Infraction>,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(None)
    }
    async fn delete_by_id(
        &self,
        _: &str,
    ) -> Result<bool, crate::sentinel::domain::errors::DomainError> {
        Ok(false)
    }
    async fn delete_older_than_days(
        &self,
        _: &str,
        _: i32,
    ) -> Result<u64, crate::sentinel::domain::errors::DomainError> {
        Ok(0)
    }
}

struct MockCache;

#[async_trait]
impl CachePort for MockCache {
    async fn get_rules(
        &self,
        _: &str,
    ) -> Result<
        Option<Vec<crate::sentinel::domain::entities::system::rule::Rule>>,
        crate::sentinel::domain::errors::DomainError,
    > {
        Ok(None)
    }
    async fn set_rules(
        &self,
        _: &str,
        _: &[crate::sentinel::domain::entities::system::rule::Rule],
    ) -> Result<(), crate::sentinel::domain::errors::DomainError> {
        Ok(())
    }
    async fn invalidate_rules(
        &self,
        _: &str,
    ) -> Result<(), crate::sentinel::domain::errors::DomainError> {
        Ok(())
    }
    async fn get_json(
        &self,
        _: &str,
    ) -> Result<Option<String>, crate::sentinel::domain::errors::DomainError> {
        Ok(None)
    }
    async fn set_json(
        &self,
        _: &str,
        _: &str,
        _: u64,
    ) -> Result<(), crate::sentinel::domain::errors::DomainError> {
        Ok(())
    }
    async fn invalidate(
        &self,
        _: &str,
    ) -> Result<(), crate::sentinel::domain::errors::DomainError> {
        Ok(())
    }
    async fn invalidate_pattern(
        &self,
        _: &str,
    ) -> Result<(), crate::sentinel::domain::errors::DomainError> {
        Ok(())
    }
}

use crate::sentinel::domain::entities::system::rule::Rule;
use crate::sentinel::domain::services::moderation::channel_tension::TensionAction;
use crate::sentinel::ports::outbound::ai::inference_service::InferenceClassification;
use chrono::Utc;
use uuid::Uuid;

fn make_rule(flag_type: FlagType, weight: f64) -> Rule {
    let now = Utc::now();
    Rule {
        id: Uuid::new_v4(),
        guild_id: "test".into(),
        flag_type,
        weight,
        threshold_warn: 2.0,
        threshold_delete: 4.0,
        threshold_mute: 6.0,
        threshold_ban: 9.0,
        enabled: true,
        created_at: now,
        updated_at: now,
    }
}

fn cls(label: &str, confidence: f32) -> InferenceClassification {
    InferenceClassification {
        label: label.to_string(),
        confidence,
    }
}

// ── resolve_thresholds ──

#[test]
fn test_resolve_thresholds_defaults() {
    let (w, d, m, b) = resolve_thresholds(&[], &[], &ScoringConfig::default());
    assert_eq!(w, 2.0);
    assert_eq!(d, 4.0);
    assert_eq!(m, 6.0);
    assert_eq!(b, 9.0);
}

#[test]
fn test_resolve_thresholds_with_rules() {
    let rules = vec![make_rule(FlagType::Spam, 3.0)];
    let (w, d, m, b) = resolve_thresholds(&rules, &[FlagType::Spam], &ScoringConfig::default());
    assert_eq!(w, 2.0);
    assert_eq!(d, 4.0);
    assert_eq!(m, 6.0);
    assert_eq!(b, 9.0);
}

#[test]
fn test_resolve_thresholds_ignores_disabled() {
    let mut rule = make_rule(FlagType::Spam, 3.0);
    rule.threshold_warn = 0.5;
    rule.enabled = false;
    let (w, _, _, _) = resolve_thresholds(&[rule], &[FlagType::Spam], &ScoringConfig::default());
    assert_eq!(w, 2.0);
}

#[test]
fn test_resolve_thresholds_takes_minimum() {
    let mut r1 = make_rule(FlagType::Spam, 3.0);
    r1.threshold_warn = 1.5;
    r1.threshold_ban = 7.0;

    let mut r2 = make_rule(FlagType::Insult, 5.0);
    r2.threshold_warn = 3.0;
    r2.threshold_ban = 10.0;

    let (w, _, _, b) = resolve_thresholds(
        &[r1, r2],
        &[FlagType::Spam, FlagType::Insult],
        &ScoringConfig::default(),
    );
    assert_eq!(w, 1.5);
    assert_eq!(b, 7.0);
}

#[test]
fn test_default_text_threshold() {
    assert_eq!(DEFAULT_TEXT_THRESHOLD, 0.5);
}

struct MockInference;
impl crate::sentinel::ports::outbound::ai::inference_service::InferenceService for MockInference {
    fn vision_available(&self) -> bool {
        false
    }
    fn text_available(&self) -> bool {
        false
    }
    fn classify_image(
        &self,
        _: ndarray::Array4<f32>,
    ) -> Result<Vec<InferenceClassification>, String> {
        Ok(vec![])
    }
    fn classify_text(
        &self,
        _: ndarray::Array2<i64>,
        _: ndarray::Array2<i64>,
    ) -> Result<Vec<InferenceClassification>, String> {
        Ok(vec![])
    }
}

struct MockTokenizer;
impl crate::sentinel::ports::outbound::ai::text_tokenizer::TextTokenizer for MockTokenizer {
    fn available(&self) -> bool {
        false
    }
    fn tokenize(&self, _: &str) -> Result<(ndarray::Array2<i64>, ndarray::Array2<i64>), String> {
        Err("mock".to_string())
    }
}

#[test]
fn test_with_text_inference_sets_fields() {
    use crate::sentinel::domain::services::ai::inference_limiter::InferenceRateLimiter;
    use std::sync::Arc;

    let inference = Arc::new(MockInference);
    let tokenizer = Arc::new(MockTokenizer);

    let _service = AnalyzeMessageService::new(
        Arc::new(MockRuleRepo),
        Arc::new(MockInfractionRepo),
        Arc::new(MockCache),
        Arc::new(MockBotConfigRepo),
        Arc::new(InferenceRateLimiter::new(4, 0)),
    )
    .with_text_inference(inference, tokenizer);
}

// ══════════════════════════════════════════════════════════
//  Tests parse_ia_config_from_bot_config
// ══════════════════════════════════════════════════════════

fn bot_entry(
    key: &str,
    value: &str,
) -> crate::sentinel::domain::entities::system::bot_config::BotGuildConfig {
    crate::sentinel::domain::entities::system::bot_config::BotGuildConfig {
        id: uuid::Uuid::new_v4(),
        guild_id: "g".into(),
        bot_name: "automod-bot".to_string(),
        config_key: key.to_string(),
        config_value: value.to_string(),
        updated_at: chrono::Utc::now(),
    }
}

#[test]
fn parse_ia_config_empty_returns_defaults() {
    let cfg = parse_ia_config_from_bot_config(&[]);
    assert!(cfg.text_enabled);
    assert!((cfg.text_threshold - 0.5).abs() < 1e-6);
    assert!((cfg.context_dampening - 0.65).abs() < 1e-6);
    assert_eq!(cfg.context_format, "natural");
}

#[test]
fn parse_ia_config_reads_all_keys() {
    let entries = vec![
        bot_entry("text_enabled", "false"),
        bot_entry("text_threshold", "0.8"),
        bot_entry("context_dampening", "0.3"),
        bot_entry("context_format", "tagged"),
    ];
    let cfg = parse_ia_config_from_bot_config(&entries);
    assert!(!cfg.text_enabled);
    assert!((cfg.text_threshold - 0.8).abs() < 1e-6);
    assert!((cfg.context_dampening - 0.3).abs() < 1e-6);
    assert_eq!(cfg.context_format, "tagged");
}

#[test]
fn parse_ia_config_clamps_out_of_range() {
    let entries = vec![
        bot_entry("text_threshold", "5.0"),
        bot_entry("context_dampening", "-1.0"),
    ];
    let cfg = parse_ia_config_from_bot_config(&entries);
    assert!((cfg.text_threshold - 1.0).abs() < 1e-6);
    assert!((cfg.context_dampening - 0.0).abs() < 1e-6);
}

#[test]
fn parse_ia_config_ignores_invalid_values_and_format() {
    let entries = vec![
        bot_entry("text_threshold", "not-a-number"),
        bot_entry("context_dampening", "abc"),
        bot_entry("context_format", "unknown"),
        bot_entry("text_enabled", "yes"),
    ];
    let cfg = parse_ia_config_from_bot_config(&entries);
    // Les cles invalides retombent sur defaut
    assert!((cfg.text_threshold - 0.5).abs() < 1e-6);
    assert!((cfg.context_dampening - 0.65).abs() < 1e-6);
    assert_eq!(cfg.context_format, "natural");
    assert!(cfg.text_enabled); // "yes" reconnu
}

// ══════════════════════════════════════════════════════════
//  Tests score_classifications — fonction pure, pas de mock
// ══════════════════════════════════════════════════════════

// ── Messages neutres ──

#[test]
fn neutral_message_returns_none() {
    let classifications = vec![
        cls("neutral", 0.95),
        cls("anger", 0.02),
        cls("rage", 0.01),
        cls("threat", 0.01),
        cls("harassment", 0.01),
    ];
    assert!(score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).is_none());
}

#[test]
fn all_below_threshold_returns_none() {
    let classifications = vec![
        cls("neutral", 0.30),
        cls("anger", 0.45),
        cls("rage", 0.10),
        cls("threat", 0.10),
        cls("harassment", 0.05),
    ];
    assert!(score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).is_none());
}

#[test]
fn empty_classifications_returns_none() {
    assert!(score_classifications(&[], &[], 0.5, &ScoringConfig::default()).is_none());
}

// ── Détection anger ──

#[test]
fn anger_above_threshold_detected() {
    let classifications = vec![
        cls("neutral", 0.20),
        cls("anger", 0.70),
        cls("rage", 0.05),
        cls("threat", 0.03),
        cls("harassment", 0.02),
    ];
    let result = score_classifications(&classifications, &[], 0.5, &ScoringConfig::default());
    assert!(result.is_some());

    let (score, flags, reason) = result.unwrap();
    assert_eq!(flags, vec![FlagType::Anger]);
    // anger weight=3.0, confidence=0.7 → 3.0 * 0.7 = 2.1
    assert!((score - 2.1).abs() < 0.01);
    assert!(reason.contains("anger"));
    assert!(reason.contains("70%"));
}

// ── Détection rage ──

#[test]
fn rage_above_threshold_detected() {
    let classifications = vec![
        cls("neutral", 0.05),
        cls("anger", 0.10),
        cls("rage", 0.80),
        cls("threat", 0.03),
        cls("harassment", 0.02),
    ];
    let (score, flags, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    assert_eq!(flags, vec![FlagType::Rage]);
    // rage weight=6.0, confidence=0.8 → 4.8
    assert!((score - 4.8).abs() < 0.01);
}

// ── Détection threat ──

#[test]
fn threat_above_threshold_detected() {
    let classifications = vec![
        cls("neutral", 0.02),
        cls("anger", 0.03),
        cls("rage", 0.05),
        cls("threat", 0.85),
        cls("harassment", 0.05),
    ];
    let (score, flags, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    assert_eq!(flags, vec![FlagType::Threat]);
    // threat weight=8.0, confidence=0.85 → 6.8
    assert!((score - 6.8).abs() < 0.01);
}

// ── Détection harassment ──

#[test]
fn harassment_above_threshold_detected() {
    let classifications = vec![
        cls("neutral", 0.05),
        cls("anger", 0.05),
        cls("rage", 0.05),
        cls("threat", 0.05),
        cls("harassment", 0.80),
    ];
    let (score, flags, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    assert_eq!(flags, vec![FlagType::Harassment]);
    // harassment weight=7.0, confidence=0.8 → 5.6
    assert!((score - 5.6).abs() < 0.01);
}

// ── Combinaisons ──

#[test]
fn anger_plus_rage_combined_score() {
    let classifications = vec![
        cls("neutral", 0.05),
        cls("anger", 0.60),
        cls("rage", 0.70),
        cls("threat", 0.03),
        cls("harassment", 0.02),
    ];
    let (score, flags, reason) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    assert_eq!(flags.len(), 2);
    assert!(flags.contains(&FlagType::Anger));
    assert!(flags.contains(&FlagType::Rage));
    // anger: 3.0*0.6=1.8 + rage: 6.0*0.7=4.2 → 6.0
    assert!((score - 6.0).abs() < 0.01);
    assert!(reason.contains("anger"));
    assert!(reason.contains("rage"));
}

#[test]
fn all_toxic_flags_combined() {
    let classifications = vec![
        cls("neutral", 0.01),
        cls("anger", 0.60),
        cls("rage", 0.70),
        cls("threat", 0.80),
        cls("harassment", 0.90),
    ];
    let (score, flags, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    assert_eq!(flags.len(), 4);
    // anger:3*0.6=1.8 + rage:6*0.7=4.2 + threat:8*0.8=6.4 + harassment:7*0.9=6.3 = 18.7
    assert!((score - 18.7).abs() < 0.01);
}

// ── Seuils personnalisés ──

#[test]
fn strict_threshold_filters_out_low_confidence() {
    let classifications = vec![cls("anger", 0.70), cls("rage", 0.85)];
    // Seuil strict = 0.8 → anger(0.7) rejeté, rage(0.85) accepté
    let (score, flags, _) =
        score_classifications(&classifications, &[], 0.8, &ScoringConfig::default()).unwrap();
    assert_eq!(flags, vec![FlagType::Rage]);
    // rage: 6.0 * 0.85 = 5.1
    assert!((score - 5.1).abs() < 0.01);
}

#[test]
fn very_strict_threshold_rejects_all() {
    let classifications = vec![cls("anger", 0.70), cls("rage", 0.80), cls("threat", 0.85)];
    // Seuil = 0.95 → tout rejeté
    assert!(
        score_classifications(&classifications, &[], 0.95, &ScoringConfig::default()).is_none()
    );
}

#[test]
fn zero_threshold_accepts_everything() {
    let classifications = vec![cls("anger", 0.01), cls("rage", 0.01)];
    let result = score_classifications(&classifications, &[], 0.0, &ScoringConfig::default());
    assert!(result.is_some());
    assert_eq!(result.unwrap().1.len(), 2);
}

#[test]
fn exact_threshold_boundary_accepted() {
    let classifications = vec![cls("anger", 0.50)];
    // confidence == threshold → accepté (>=)
    let result = score_classifications(&classifications, &[], 0.5, &ScoringConfig::default());
    assert!(result.is_some());
}

#[test]
fn just_below_threshold_rejected() {
    let classifications = vec![cls("anger", 0.499)];
    assert!(score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).is_none());
}

// ── Règles custom ──

#[test]
fn custom_rule_overrides_default_weight() {
    let classifications = vec![cls("anger", 0.80)];
    let rules = vec![make_rule(FlagType::Anger, 10.0)];
    let (score, _, _) =
        score_classifications(&classifications, &rules, 0.5, &ScoringConfig::default()).unwrap();
    // custom weight=10.0, confidence=0.8 → 8.0 (vs 2.4 par défaut)
    assert!((score - 8.0).abs() < 0.01);
}

#[test]
fn disabled_rule_uses_default_weight() {
    let classifications = vec![cls("anger", 0.80)];
    let mut rule = make_rule(FlagType::Anger, 10.0);
    rule.enabled = false;
    let (score, _, _) =
        score_classifications(&classifications, &[rule], 0.5, &ScoringConfig::default()).unwrap();
    // rule disabled → default weight=3.0, confidence=0.8 → 2.4
    assert!((score - 2.4).abs() < 0.01);
}

#[test]
fn custom_rule_for_different_flag_no_effect() {
    let classifications = vec![cls("anger", 0.80)];
    let rules = vec![make_rule(FlagType::Rage, 15.0)];
    let (score, _, _) =
        score_classifications(&classifications, &rules, 0.5, &ScoringConfig::default()).unwrap();
    // rule est pour Rage, pas Anger → default anger weight=3.0
    assert!((score - 2.4).abs() < 0.01);
}

#[test]
fn multiple_custom_rules_applied() {
    let classifications = vec![cls("anger", 0.60), cls("threat", 0.70)];
    let rules = vec![
        make_rule(FlagType::Anger, 5.0),
        make_rule(FlagType::Threat, 12.0),
    ];
    let (score, _, _) =
        score_classifications(&classifications, &rules, 0.5, &ScoringConfig::default()).unwrap();
    // anger: 5.0*0.6=3.0 + threat: 12.0*0.7=8.4 → 11.4
    assert!((score - 11.4).abs() < 0.01);
}

// ── Labels non reconnus ignorés ──

#[test]
fn unknown_labels_ignored() {
    let classifications = vec![cls("neutral", 0.90), cls("joy", 0.80), cls("sadness", 0.70)];
    assert!(score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).is_none());
}

// ── Format de la raison ──

#[test]
fn reason_format_single_flag() {
    let classifications = vec![cls("threat", 0.90)];
    let (_, _, reason) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    assert_eq!(reason, "IA sentiment : threat(90%)");
}

#[test]
fn reason_format_multiple_flags() {
    let classifications = vec![cls("anger", 0.70), cls("harassment", 0.80)];
    let (_, _, reason) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    assert_eq!(reason, "IA sentiment : anger(70%), harassment(80%)");
}

// ══════════════════════════════════════════════════════════
//  Tests de scoring combiné → action
// ══════════════════════════════════════════════════════════

#[test]
fn anger_only_triggers_warn() {
    // anger: weight=3.0, confidence=0.8 → score=2.4 >= warn(2.0) mais < delete(4.0)
    let classifications = vec![cls("anger", 0.80)];
    let (score, _, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    let (t_warn, t_delete, _, _) = resolve_thresholds(&[], &[], &ScoringConfig::default());
    assert!(score >= t_warn);
    assert!(score < t_delete);
}

#[test]
fn rage_triggers_delete_or_mute() {
    // rage: weight=6.0, confidence=0.85 → score=5.1 >= delete(4.0) mais < mute(6.0)
    let classifications = vec![cls("rage", 0.85)];
    let (score, _, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    let (_, t_delete, t_mute, _) = resolve_thresholds(&[], &[], &ScoringConfig::default());
    assert!(score >= t_delete);
    assert!(score < t_mute);
}

#[test]
fn threat_high_confidence_triggers_mute() {
    // threat: weight=8.0, confidence=0.90 → score=7.2 >= mute(6.0) mais < ban(9.0)
    let classifications = vec![cls("threat", 0.90)];
    let (score, _, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    let (_, _, t_mute, t_ban) = resolve_thresholds(&[], &[], &ScoringConfig::default());
    assert!(score >= t_mute);
    assert!(score < t_ban);
}

#[test]
fn rage_plus_threat_triggers_ban() {
    // rage:6.0*0.8=4.8 + threat:8.0*0.8=6.4 → 11.2 >= ban(9.0)
    let classifications = vec![cls("rage", 0.80), cls("threat", 0.80)];
    let (score, _, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    let (_, _, _, t_ban) = resolve_thresholds(&[], &[], &ScoringConfig::default());
    assert!(score >= t_ban);
}

// ── C5 : cap anti first-message auto-ban ──

#[test]
fn ia_induced_ban_is_capped_to_mute() {
    // Score bot seul (3.0) < seuil ban (9.0), action combinée = Ban -> Mute.
    let (action, duration) = cap_ia_induced_ban(Action::Ban, None, 3.0, 9.0, 600);
    assert_eq!(action, Action::Mute);
    assert_eq!(duration, Some(600));
}

#[test]
fn bot_only_ban_is_preserved() {
    // Score bot seul (10.0) >= seuil ban : le Ban auto bot-seul est conservé.
    let (action, duration) = cap_ia_induced_ban(Action::Ban, None, 10.0, 9.0, 600);
    assert_eq!(action, Action::Ban);
    assert_eq!(duration, None);
}

#[test]
fn non_ban_actions_are_untouched() {
    let (action, duration) = cap_ia_induced_ban(Action::Mute, Some(300), 1.0, 9.0, 600);
    assert_eq!(action, Action::Mute);
    assert_eq!(duration, Some(300));
    let (action, _) = cap_ia_induced_ban(Action::Warn, None, 1.0, 9.0, 600);
    assert_eq!(action, Action::Warn);
}

#[test]
fn anger_low_confidence_below_warn() {
    // anger: weight=3.0, confidence=0.55 → score=1.65 < warn(2.0)
    let classifications = vec![cls("anger", 0.55)];
    let (score, _, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    let (t_warn, _, _, _) = resolve_thresholds(&[], &[], &ScoringConfig::default());
    assert!(score < t_warn);
}

// ══════════════════════════════════════════════════════════
//  Tests avec confidences réalistes (somme ~1.0 via softmax)
// ══════════════════════════════════════════════════════════

#[test]
fn realistic_softmax_angry_message() {
    // Softmax distribution typique d'un message colérique
    let classifications = vec![
        cls("neutral", 0.15),
        cls("anger", 0.55),
        cls("rage", 0.15),
        cls("threat", 0.10),
        cls("harassment", 0.05),
    ];
    let (score, flags, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    assert_eq!(flags, vec![FlagType::Anger]);
    // anger: 3.0 * 0.55 = 1.65
    assert!((score - 1.65).abs() < 0.01);
}

#[test]
fn realistic_softmax_threat_message() {
    // Message de menace directe
    let classifications = vec![
        cls("neutral", 0.02),
        cls("anger", 0.08),
        cls("rage", 0.10),
        cls("threat", 0.75),
        cls("harassment", 0.05),
    ];
    let (score, flags, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    assert_eq!(flags, vec![FlagType::Threat]);
    // threat: 8.0 * 0.75 = 6.0
    assert!((score - 6.0).abs() < 0.01);
}

#[test]
fn realistic_softmax_harassment_escalation() {
    // Harcèlement avec rage sous-jacente
    let classifications = vec![
        cls("neutral", 0.03),
        cls("anger", 0.07),
        cls("rage", 0.55),
        cls("threat", 0.05),
        cls("harassment", 0.30),
    ];
    // threshold 0.5 → rage(0.55) detecté, harassment(0.30) rejeté
    let (score, flags, _) =
        score_classifications(&classifications, &[], 0.5, &ScoringConfig::default()).unwrap();
    assert_eq!(flags, vec![FlagType::Rage]);
    // rage: 6.0 * 0.55 = 3.3
    assert!((score - 3.3).abs() < 0.01);
}

#[test]
fn realistic_softmax_harassment_escalation_lower_threshold() {
    let classifications = vec![
        cls("neutral", 0.03),
        cls("anger", 0.07),
        cls("rage", 0.55),
        cls("threat", 0.05),
        cls("harassment", 0.30),
    ];
    // Seuil plus bas (0.25) → rage ET harassment détectés
    let (score, flags, _) =
        score_classifications(&classifications, &[], 0.25, &ScoringConfig::default()).unwrap();
    assert_eq!(flags.len(), 2);
    // rage: 6.0*0.55=3.3 + harassment: 7.0*0.30=2.1 → 5.4
    assert!((score - 5.4).abs() < 0.01);
}

// ══════════════════════════════════════════════════════════
//  Tests build_contextual_content
// ══════════════════════════════════════════════════════════

fn ctx_msg(
    username: &str,
    content: &str,
) -> crate::sentinel::ports::inbound::ai::analyze_message::ContextMessageEntry {
    crate::sentinel::ports::inbound::ai::analyze_message::ContextMessageEntry {
        username: username.to_string(),
        content: content.to_string(),
    }
}

#[test]
fn context_empty_returns_content_only() {
    let result = build_contextual_content("hello", &[], "natural");
    assert_eq!(result, "hello");
}

#[test]
fn context_natural_format() {
    let ctx = vec![ctx_msg("Alice", "salut"), ctx_msg("Bob", "ca va ?")];
    let result = build_contextual_content("oui bien", &ctx, "natural");
    assert!(result.contains("Alice: salut"));
    assert!(result.contains("Bob: ca va ?"));
    assert!(result.contains("---"));
    assert!(result.ends_with("oui bien"));
}

#[test]
fn context_tagged_format() {
    let ctx = vec![ctx_msg("Alice", "salut")];
    let result = build_contextual_content("oui", &ctx, "tagged");
    assert!(result.starts_with("[message] oui [/message]"));
    assert!(result.contains("[context] Alice: salut [/context]"));
}

#[test]
fn context_unknown_format_defaults_to_natural() {
    let ctx = vec![ctx_msg("X", "y")];
    let result = build_contextual_content("z", &ctx, "unknown");
    assert!(result.contains("---")); // natural format
}

// ══════════════════════════════════════════════════════════
//  Tests parse_tension_config
// ══════════════════════════════════════════════════════════

#[test]
fn parse_tension_config_defaults_when_empty() {
    let cfg = parse_tension_config(&[]);
    assert!(!cfg.enabled);
    assert_eq!(cfg.buffer_size, 5);
    assert_eq!(cfg.threshold_warn, 3.0);
    assert_eq!(cfg.threshold_delete, 5.0);
    assert_eq!(cfg.threshold_mute, 7.0);
    assert_eq!(cfg.mute_duration_secs, 300);
}

#[test]
fn deepseek_toxicity_is_weighted_with_the_detected_rule() {
    use crate::sentinel::ports::outbound::ai::deepseek_moderation_service::DeepSeekModerationAnalysis;

    let analysis = DeepSeekModerationAnalysis {
        toxicity_score: 0.8,
        sentiment: "threat".to_string(),
        flags: vec![],
        recommended_action: "warn".to_string(),
        reason: "Menace explicite".to_string(),
    };

    let (score, flags, _) = score_deepseek_analysis(&analysis, &[], 0.5, &ScoringConfig::default())
        .expect("une menace DeepSeek doit produire un score");

    assert_eq!(flags, vec![FlagType::Threat]);
    assert!((score - 6.4).abs() < 0.01, "poids menace 8 × confiance 0.8");
}

#[test]
fn deepseek_unknown_toxic_label_falls_back_to_weighted_harassment() {
    use crate::sentinel::ports::outbound::ai::deepseek_moderation_service::DeepSeekModerationAnalysis;

    let analysis = DeepSeekModerationAnalysis {
        toxicity_score: 0.9,
        sentiment: "inappropriate".to_string(),
        flags: vec![],
        recommended_action: "warn".to_string(),
        reason: "Contenu inapproprié détecté".to_string(),
    };

    let (score, flags, _) = score_deepseek_analysis(&analysis, &[], 0.5, &ScoringConfig::default())
        .expect("un signal toxique inconnu ne doit jamais donner 0");

    assert_eq!(flags, vec![FlagType::Harassment]);
    assert!(
        (score - 6.3).abs() < 0.01,
        "poids harcelement 7 × confiance 0.9"
    );
}

#[test]
fn parse_tension_config_reads_all_keys() {
    let entries = vec![
        bot_entry("channel_tension_enabled", "true"),
        bot_entry("channel_tension_buffer_size", "10"),
        bot_entry("channel_tension_threshold_warn", "2.0"),
        bot_entry("channel_tension_threshold_delete", "4.5"),
        bot_entry("channel_tension_threshold_mute", "8.0"),
        bot_entry("channel_tension_mute_duration_secs", "600"),
    ];
    let cfg = parse_tension_config(&entries);
    assert!(cfg.enabled);
    assert_eq!(cfg.buffer_size, 10);
    assert_eq!(cfg.threshold_warn, 2.0);
    assert_eq!(cfg.threshold_delete, 4.5);
    assert_eq!(cfg.threshold_mute, 8.0);
    assert_eq!(cfg.mute_duration_secs, 600);
}

#[test]
fn parse_tension_config_rejects_buffer_size_zero() {
    let entries = vec![bot_entry("channel_tension_buffer_size", "0")];
    let cfg = parse_tension_config(&entries);
    assert_eq!(cfg.buffer_size, 5); // defaut conserve
}

#[test]
fn parse_tension_config_ignores_malformed_values() {
    let entries = vec![
        bot_entry("channel_tension_enabled", "nope"),
        bot_entry("channel_tension_buffer_size", "abc"),
        bot_entry("channel_tension_threshold_warn", "not_a_number"),
        bot_entry("channel_tension_mute_duration_secs", "bad"),
    ];
    let cfg = parse_tension_config(&entries);
    assert!(!cfg.enabled);
    assert_eq!(cfg.buffer_size, 5);
    assert_eq!(cfg.threshold_warn, 3.0);
    assert_eq!(cfg.mute_duration_secs, 300);
}

// ══════════════════════════════════════════════════════════
//  Tests tension_is_stronger (ordre de severite)
// ══════════════════════════════════════════════════════════

#[test]
fn tension_is_stronger_ban_not_overridden() {
    // Aucun TensionAction ne peut depasser Ban (4).
    assert!(!tension_is_stronger(&Action::Ban, TensionAction::Mute));
    assert!(!tension_is_stronger(&Action::Ban, TensionAction::None));
}

#[test]
fn tension_is_stronger_escalates_from_none() {
    assert!(tension_is_stronger(&Action::None, TensionAction::Warn));
    assert!(tension_is_stronger(&Action::None, TensionAction::Delete));
    assert!(tension_is_stronger(&Action::None, TensionAction::Mute));
    assert!(!tension_is_stronger(&Action::None, TensionAction::None));
}

#[test]
fn tension_is_stronger_equal_severity_returns_false() {
    assert!(!tension_is_stronger(&Action::Warn, TensionAction::Warn));
    assert!(!tension_is_stronger(&Action::Mute, TensionAction::Mute));
}

#[test]
fn tension_is_stronger_downgrade_returns_false() {
    assert!(!tension_is_stronger(&Action::Mute, TensionAction::Warn));
    assert!(!tension_is_stronger(&Action::Delete, TensionAction::Warn));
}
