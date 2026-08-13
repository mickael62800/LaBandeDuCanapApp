//! Tests de la decision de routage automod (fonction pure `decide`).

use super::*;
use crate::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::sentinel::domain::enums::moderation::action::Action;

fn flags(spam: bool, insult: bool, link: bool, phishing: bool) -> DetectionFlags {
    DetectionFlags {
        spam,
        insult,
        profanity: false,
        link,
        phishing,
    }
}

/// Entrees "neutres" : rien de special, personnalisables par test.
fn base<'a>(f: &'a DetectionFlags, content: &'a str) -> RoutingInputs<'a> {
    RoutingInputs {
        flags: f,
        content,
        score: 0.0,
        action: Action::None,
        human_only: false,
        auto_protect: false,
        auto_delete_links: false,
        selective_auto_actions: false,
        auto_warn: true,
        auto_delete: true,
        auto_mute: true,
        auto_kick: false,
        auto_ban: false,
        ai_review_mode: false,
        review_min_score: 0.5,
        log_channel_set: false,
    }
}

// ── contains_discord_invite ──────────────────────────────────────────────

#[test]
fn discord_invite_variants_detected() {
    assert!(contains_discord_invite("rejoins discord.gg/abc"));
    assert!(contains_discord_invite("DISCORD.COM/INVITE/xyz"));
    assert!(contains_discord_invite("discordapp.com/invite/foo"));
}

#[test]
fn discord_invite_absent() {
    assert!(!contains_discord_invite("hello world"));
    assert!(!contains_discord_invite("visit discord.com/channels/1/2"));
}

// ── is_severe_content ─────────────────────────────────────────────────────

#[test]
fn severe_content_phishing_or_invite() {
    let f = flags(false, false, false, true);
    assert!(is_severe_content(&f, "peu importe"));
    let f2 = flags(false, false, false, false);
    assert!(is_severe_content(&f2, "join discord.gg/x"));
    assert!(!is_severe_content(&f2, "rien de special"));
}

// ── contains_non_image_url ───────────────────────────────────────────────

#[test]
fn non_image_url_true_for_generic_link() {
    assert!(contains_non_image_url("regarde https://evil.example/page"));
    assert!(contains_non_image_url("http://foo.bar"));
}

#[test]
fn non_image_url_false_for_image_link() {
    assert!(!contains_non_image_url("https://cdn.site/pic.png"));
    assert!(!contains_non_image_url("http://x/y.JPG"));
    assert!(!contains_non_image_url("https://x/a.gif?width=10"));
}

#[test]
fn non_image_url_false_without_url() {
    assert!(!contains_non_image_url("just some text, no link"));
    assert!(!contains_non_image_url("ftp://not-http/x"));
}

#[test]
fn non_image_url_strips_trailing_punctuation() {
    // Une URL suivie d'une virgule doit rester detectee.
    assert!(contains_non_image_url("va sur https://foo.bar/page, merci"));
}

// ── decide : cas severe ──────────────────────────────────────────────────

#[test]
fn decide_severe_requires_auto_protect() {
    let f = flags(false, false, false, true);
    let mut i = base(&f, "phishing ici");
    // sans auto_protect -> pas severe.
    assert!(!decide(&i).severe);
    // avec auto_protect -> severe.
    i.auto_protect = true;
    assert!(decide(&i).severe);
}

#[test]
fn decide_severe_cards_when_log_channel_set() {
    let f = flags(false, false, false, true);
    let mut i = base(&f, "phishing");
    i.auto_protect = true;
    i.log_channel_set = true;
    let d = decide(&i);
    assert!(d.severe);
    assert_eq!(d.route, Routing::Card);
}

// ── decide : lien generique ──────────────────────────────────────────────

#[test]
fn decide_generic_link_defaults_to_card() {
    let f = flags(false, false, true, false);
    let mut i = base(&f, "regarde https://evil.example/x");
    i.log_channel_set = true;
    let d = decide(&i);
    assert_eq!(d.route, Routing::Card);
    assert!(!d.auto_delete_link);
}

#[test]
fn decide_generic_link_auto_delete_when_opt_in() {
    let f = flags(false, false, true, false);
    let mut i = base(&f, "regarde https://evil.example/x");
    i.log_channel_set = true;
    i.auto_delete_links = true;
    let d = decide(&i);
    assert!(d.auto_delete_link);
    // suppression seche : pas d'autre action.
    assert_eq!(d.route, Routing::None);
}

#[test]
fn decide_link_flag_but_image_url_not_generic() {
    // flag link mais l'URL est une image -> pas de suppression, pas severe.
    let f = flags(false, false, true, false);
    let mut i = base(&f, "https://cdn/x.png");
    i.log_channel_set = true;
    i.auto_delete_links = true;
    let d = decide(&i);
    assert!(!d.auto_delete_link);
}

// ── decide : human_only ──────────────────────────────────────────────────

#[test]
fn decide_human_only_without_channel_is_none() {
    let f = flags(true, false, false, false);
    let mut i = base(&f, "spam");
    i.human_only = true;
    i.action = Action::Ban;
    // pas de log_channel -> pas de carte -> aucune action auto.
    assert_eq!(decide(&i).route, Routing::None);
}

#[test]
fn decide_human_only_with_channel_cards() {
    let f = flags(true, false, false, false);
    let mut i = base(&f, "spam");
    i.human_only = true;
    i.log_channel_set = true;
    i.score = 3.0;
    assert_eq!(decide(&i).route, Routing::Card);
}

#[test]
fn decide_human_only_with_zero_score_does_not_create_an_empty_card() {
    let f = flags(false, false, false, false);
    let mut i = base(&f, "message normal");
    i.human_only = true;
    i.log_channel_set = true;
    assert_eq!(decide(&i).route, Routing::None);
}

#[test]
fn decide_unapproved_auto_action_is_sent_to_review_card() {
    let f = flags(false, true, false, false);
    let mut i = base(&f, "insulte");
    i.action = Action::Mute;
    i.score = 6.0;
    i.log_channel_set = true;
    i.selective_auto_actions = true;
    i.auto_mute = false;
    assert_eq!(decide(&i).route, Routing::Card);
}

#[test]
fn decide_approved_auto_action_is_applied() {
    let f = flags(false, true, false, false);
    let mut i = base(&f, "insulte");
    i.action = Action::Mute;
    i.score = 6.0;
    i.selective_auto_actions = true;
    i.auto_mute = true;
    let decision = decide(&i);
    assert_eq!(decision.route, Routing::Auto);
    assert!(decision.auto_action);
}

#[test]
fn selective_auto_action_keeps_a_review_card_when_review_is_enabled() {
    let f = flags(false, true, false, false);
    let mut i = base(&f, "insulte");
    i.action = Action::Mute;
    i.score = 6.0;
    i.log_channel_set = true;
    i.ai_review_mode = true;
    i.selective_auto_actions = true;
    i.auto_mute = true;

    let decision = decide(&i);
    assert_eq!(decision.route, Routing::Card);
    assert!(decision.auto_action);
}

#[test]
fn cap_ban_to_mute_when_mute_is_the_only_allowed_auto_action() {
    assert_eq!(
        cap_to_allowed_auto_action(&Action::Ban, true, false, false, true, false, false),
        Action::Mute
    );
}

#[test]
fn cap_never_escalates_a_lower_unapproved_action() {
    assert_eq!(
        cap_to_allowed_auto_action(&Action::Warn, true, false, false, true, false, false),
        Action::Warn
    );
}

// ── decide : ai_review_mode + seuil ──────────────────────────────────────

#[test]
fn decide_ai_review_at_threshold_cards() {
    let f = flags(true, false, false, false);
    let mut i = base(&f, "spam");
    i.log_channel_set = true;
    i.ai_review_mode = true;
    i.review_min_score = 0.5;
    // exactement au seuil -> >= -> carte.
    i.score = 0.5;
    assert_eq!(decide(&i).route, Routing::Card);
    // juste en dessous -> pas de carte ; action None -> None.
    i.score = 0.49;
    assert_eq!(decide(&i).route, Routing::None);
}

// ── decide : mode auto ───────────────────────────────────────────────────

#[test]
fn decide_auto_when_action_set_no_card() {
    let f = flags(true, false, false, false);
    let mut i = base(&f, "spam");
    i.action = Action::Mute;
    // pas human_only, pas de carte -> action auto.
    assert_eq!(decide(&i).route, Routing::Auto);
}

#[test]
fn decide_none_when_action_none_no_card() {
    let f = flags(true, false, false, false);
    let i = base(&f, "spam");
    assert_eq!(decide(&i).route, Routing::None);
}
