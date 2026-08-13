use super::*;
use platform_core::sentinel::domain::entities::moderation::copilot::MemberModerationContext;
use platform_core::sentinel::domain::entities::moderation::copilot::PrecedentDistribution;
use platform_core::sentinel::domain::entities::moderation::copilot::SanctionSuggestion;
use platform_core::sentinel::domain::entities::moderation::copilot::SuggestionBasis;
use platform_core::sentinel::domain::entities::moderation::review::automod::AppliedAction;

#[test]
fn mappe_le_contexte_domaine_vers_dto() {
    let ctx = MemberModerationContext {
        active_strikes: 2,
        sanctions_by_type: vec![("warn".into(), 3)],
        last_sanction_at: None,
        open_reviews: 1,
        precedents: PrecedentDistribution {
            flag_category: "spam".into(),
            counts_by_action: vec![("mute".into(), 4)],
            total: 4,
        },
        suggestion: SanctionSuggestion {
            action: Some(AppliedAction::Mute),
            basis: SuggestionBasis::Both,
            rationale: "raison".into(),
            precedent_count: 4,
        },
    };
    let dto = MemberModerationContextDto::from(ctx);
    assert_eq!(dto.active_strikes, 2);
    assert_eq!(dto.suggestion.action.as_deref(), Some("mute"));
    assert_eq!(dto.suggestion.basis, "both");
    assert_eq!(dto.precedents.total, 4);
    assert_eq!(dto.sanctions_by_type[0].action, "warn");
}

#[test]
fn suggestion_sans_action_donne_null() {
    let s = SanctionSuggestion {
        action: None,
        basis: SuggestionBasis::Insufficient,
        rationale: "rien".into(),
        precedent_count: 0,
    };
    let dto = SanctionSuggestionDto::from(s);
    assert!(dto.action.is_none());
    assert_eq!(dto.basis, "insufficient");
}
