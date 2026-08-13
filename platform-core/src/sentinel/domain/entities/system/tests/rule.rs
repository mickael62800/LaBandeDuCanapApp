use super::*;
use crate::sentinel::domain::enums::moderation::flag_type::FlagType;

#[test]
fn new_sets_default_weight_matching_flag_type() {
    assert_eq!(Rule::new("g".into(), FlagType::Spam).weight, 3.0);
    assert_eq!(Rule::new("g".into(), FlagType::Insult).weight, 5.0);
    assert_eq!(Rule::new("g".into(), FlagType::Link).weight, 1.0);
    assert_eq!(Rule::new("g".into(), FlagType::Phishing).weight, 7.0);
    assert_eq!(Rule::new("g".into(), FlagType::Nsfw).weight, 8.0);
    assert_eq!(Rule::new("g".into(), FlagType::Illicit).weight, 9.0);
    assert_eq!(Rule::new("g".into(), FlagType::Anger).weight, 3.0);
    assert_eq!(Rule::new("g".into(), FlagType::Rage).weight, 6.0);
    assert_eq!(Rule::new("g".into(), FlagType::Threat).weight, 8.0);
    assert_eq!(Rule::new("g".into(), FlagType::Harassment).weight, 7.0);
}

#[test]
fn new_sets_default_thresholds() {
    let r = Rule::new("g".into(), FlagType::Spam);
    assert_eq!(r.threshold_warn, 2.0);
    assert_eq!(r.threshold_delete, 4.0);
    assert_eq!(r.threshold_mute, 6.0);
    assert_eq!(r.threshold_ban, 9.0);
}

#[test]
fn new_thresholds_are_strictly_increasing() {
    let r = Rule::new("g".into(), FlagType::Spam);
    assert!(r.threshold_warn < r.threshold_delete);
    assert!(r.threshold_delete < r.threshold_mute);
    assert!(r.threshold_mute < r.threshold_ban);
}

#[test]
fn new_enabled_by_default() {
    assert!(Rule::new("g".into(), FlagType::Spam).enabled);
}

#[test]
fn new_generates_unique_ids() {
    let r1 = Rule::new("g".into(), FlagType::Spam);
    let r2 = Rule::new("g".into(), FlagType::Spam);
    assert_ne!(r1.id, r2.id);
}

#[test]
fn new_copies_guild_id() {
    let r = Rule::new("my_guild_123".into(), FlagType::Spam);
    assert_eq!(r.guild_id, GuildId::new("my_guild_123"));
}

#[test]
fn new_created_at_equals_updated_at() {
    let r = Rule::new("g".into(), FlagType::Spam);
    assert_eq!(r.created_at, r.updated_at);
}

#[test]
fn weight_gradient_aligns_with_severity() {
    // Invariant metier : un flag plus grave a un poids plus eleve.
    let link = Rule::new("g".into(), FlagType::Link).weight;
    let spam = Rule::new("g".into(), FlagType::Spam).weight;
    let insult = Rule::new("g".into(), FlagType::Insult).weight;
    let phishing = Rule::new("g".into(), FlagType::Phishing).weight;
    let illicit = Rule::new("g".into(), FlagType::Illicit).weight;
    assert!(link < spam);
    assert!(spam < insult);
    assert!(insult < phishing);
    assert!(phishing < illicit);
}
