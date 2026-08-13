use super::*;

// ── SuggestedAction : roundtrip, severity ordering ───────────────────

#[test]
fn suggested_action_roundtrip_all_variants() {
    for a in [
        SuggestedAction::Warn,
        SuggestedAction::Delete,
        SuggestedAction::Mute,
        SuggestedAction::Ban,
    ] {
        let s = a.as_str();
        assert_eq!(SuggestedAction::from_str(s), Some(a.clone()));
    }
}

#[test]
fn suggested_action_from_str_invalid() {
    assert_eq!(SuggestedAction::from_str("bogus"), None);
    assert_eq!(SuggestedAction::from_str(""), None);
    // Sensible a la casse.
    assert_eq!(SuggestedAction::from_str("Ban"), None);
    // `prevention`/`ignore` ne sont PAS des SuggestedAction.
    assert_eq!(SuggestedAction::from_str("prevention"), None);
    assert_eq!(SuggestedAction::from_str("ignore"), None);
}

#[test]
fn suggested_action_severity_strictly_increasing() {
    let order = [
        SuggestedAction::Warn,
        SuggestedAction::Delete,
        SuggestedAction::Mute,
        SuggestedAction::Ban,
    ];
    for w in order.windows(2) {
        assert!(
            w[0].severity() < w[1].severity(),
            "{:?} devrait etre moins severe que {:?}",
            w[0],
            w[1]
        );
    }
    assert_eq!(SuggestedAction::Warn.severity(), 1);
    assert_eq!(SuggestedAction::Ban.severity(), 4);
}

// ── more_severe_suggested ─────────────────────────────────────────────

#[test]
fn more_severe_picks_higher_rank() {
    assert_eq!(more_severe_suggested("warn", "ban"), "ban");
    assert_eq!(more_severe_suggested("ban", "warn"), "ban");
    assert_eq!(more_severe_suggested("delete", "mute"), "mute");
    assert_eq!(more_severe_suggested("mute", "delete"), "mute");
}

#[test]
fn more_severe_equal_returns_a() {
    // Egalite de rang -> `a` est retenu (rank(a) >= rank(b)).
    assert_eq!(more_severe_suggested("mute", "mute"), "mute");
}

#[test]
fn more_severe_unknown_falls_back() {
    // `a` inconnu (rank 0) et `b` connu -> b l'emporte.
    assert_eq!(more_severe_suggested("???", "warn"), "warn");
    // `b` inconnu -> a l'emporte.
    assert_eq!(more_severe_suggested("ban", "???"), "ban");
    // Les deux inconnus -> rank egaux (0), rank(a)==0 -> "warn".
    assert_eq!(more_severe_suggested("foo", "bar"), "warn");
    assert_eq!(more_severe_suggested("", ""), "warn");
}

// ── AppliedAction : roundtrip + severity totality/ordering ────────────

#[test]
fn applied_action_roundtrip_all_variants() {
    for a in [
        AppliedAction::Prevention,
        AppliedAction::Warn,
        AppliedAction::Delete,
        AppliedAction::Mute,
        AppliedAction::Ban,
        AppliedAction::Ignore,
    ] {
        assert_eq!(AppliedAction::from_str(a.as_str()), Some(a.clone()));
    }
}

#[test]
fn applied_action_from_str_invalid() {
    assert_eq!(AppliedAction::from_str("nope"), None);
    assert_eq!(AppliedAction::from_str(""), None);
    assert_eq!(AppliedAction::from_str("BAN"), None);
}

#[test]
fn applied_action_severity_total_order() {
    // ignore(0) < prevention(1) < warn(2) < delete(3) < mute(4) < ban(5).
    let order = [
        AppliedAction::Ignore,
        AppliedAction::Prevention,
        AppliedAction::Warn,
        AppliedAction::Delete,
        AppliedAction::Mute,
        AppliedAction::Ban,
    ];
    for (i, a) in order.iter().enumerate() {
        assert_eq!(a.severity() as usize, i);
    }
    for w in order.windows(2) {
        assert!(w[0].severity() < w[1].severity());
    }
}

// ── TieAction::from_str ───────────────────────────────────────────────

#[test]
fn tie_action_from_str() {
    assert_eq!(TieAction::from_str("clemente"), TieAction::Clemente);
    assert_eq!(TieAction::from_str("severe"), TieAction::Severe);
    // Tout le reste (incl. "ignore", vide, inconnu) -> Ignore.
    assert_eq!(TieAction::from_str("ignore"), TieAction::Ignore);
    assert_eq!(TieAction::from_str(""), TieAction::Ignore);
    assert_eq!(TieAction::from_str("Severe"), TieAction::Ignore);
}

// ── tally_votes : quorum, majorite, tie-break ─────────────────────────

#[test]
fn tally_no_votes_is_ignore_quorum_unmet() {
    let r = tally_votes(&[], 3, TieAction::Severe);
    assert_eq!(r.decided, AppliedAction::Ignore);
    assert!(!r.quorum_met);
    assert_eq!(r.total_votes, 0);
}

#[test]
fn tally_below_quorum_ignored() {
    let votes = vec![AppliedAction::Ban, AppliedAction::Ban];
    let r = tally_votes(&votes, 3, TieAction::Severe);
    assert_eq!(r.decided, AppliedAction::Ignore);
    assert!(!r.quorum_met);
    assert_eq!(r.total_votes, 2);
}

#[test]
fn tally_exactly_at_quorum_counts() {
    // Pile au quorum -> depouille normalement.
    let votes = vec![
        AppliedAction::Mute,
        AppliedAction::Mute,
        AppliedAction::Warn,
    ];
    let r = tally_votes(&votes, 3, TieAction::Ignore);
    assert!(r.quorum_met);
    assert_eq!(r.decided, AppliedAction::Mute);
    assert_eq!(r.total_votes, 3);
}

#[test]
fn tally_quorum_zero_treated_as_one() {
    // quorum 0 -> max(1) : un seul vote suffit, mais zero vote reste ignore.
    let r0 = tally_votes(&[], 0, TieAction::Severe);
    assert!(!r0.quorum_met);
    let r1 = tally_votes(&[AppliedAction::Warn], 0, TieAction::Severe);
    assert!(r1.quorum_met);
    assert_eq!(r1.decided, AppliedAction::Warn);
}

#[test]
fn tally_clear_majority_wins() {
    let votes = vec![
        AppliedAction::Ban,
        AppliedAction::Ban,
        AppliedAction::Ban,
        AppliedAction::Warn,
    ];
    let r = tally_votes(&votes, 1, TieAction::Ignore);
    assert_eq!(r.decided, AppliedAction::Ban);
    assert!(r.quorum_met);
}

#[test]
fn tally_tie_ignore() {
    let votes = vec![AppliedAction::Ban, AppliedAction::Warn];
    let r = tally_votes(&votes, 1, TieAction::Ignore);
    assert_eq!(r.decided, AppliedAction::Ignore);
    assert!(r.quorum_met);
}

#[test]
fn tally_tie_clemente_picks_lowest_severity() {
    // Ban(5) vs Warn(2) a egalite -> clemente = Warn.
    let votes = vec![AppliedAction::Ban, AppliedAction::Warn];
    let r = tally_votes(&votes, 1, TieAction::Clemente);
    assert_eq!(r.decided, AppliedAction::Warn);
}

#[test]
fn tally_tie_severe_picks_highest_severity() {
    // Mute(4) vs Delete(3) vs Warn(2) a 1 chacun -> severe = Mute.
    let votes = vec![
        AppliedAction::Mute,
        AppliedAction::Delete,
        AppliedAction::Warn,
    ];
    let r = tally_votes(&votes, 1, TieAction::Severe);
    assert_eq!(r.decided, AppliedAction::Mute);
}

#[test]
fn tally_tie_clemente_three_way() {
    let votes = vec![
        AppliedAction::Ban,
        AppliedAction::Mute,
        AppliedAction::Ignore,
    ];
    let r = tally_votes(&votes, 1, TieAction::Clemente);
    assert_eq!(r.decided, AppliedAction::Ignore);
}

// ── Roles / permissions ───────────────────────────────────────────────

#[test]
fn is_moderator_requires_one_of_the_flags() {
    assert!(!is_moderator(&ModeratorFacts::default()));
    assert!(is_moderator(&ModeratorFacts {
        is_admin: true,
        ..Default::default()
    }));
    assert!(is_moderator(&ModeratorFacts {
        has_moderate_members: true,
        ..Default::default()
    }));
    assert!(is_moderator(&ModeratorFacts {
        has_manage_messages: true,
        ..Default::default()
    }));
    assert!(is_moderator(&ModeratorFacts {
        has_mod_role: true,
        ..Default::default()
    }));
    // Le role admin seul ne rend PAS moderateur (is_moderator ne le teste pas).
    assert!(!is_moderator(&ModeratorFacts {
        has_admin_role: true,
        ..Default::default()
    }));
}

#[test]
fn can_finalize_review_admins_only() {
    assert!(!can_finalize_review(&ModeratorFacts::default()));
    assert!(can_finalize_review(&ModeratorFacts {
        is_admin: true,
        ..Default::default()
    }));
    assert!(can_finalize_review(&ModeratorFacts {
        has_admin_role: true,
        ..Default::default()
    }));
    // Un simple moderateur ne peut PAS finaliser.
    assert!(!can_finalize_review(&ModeratorFacts {
        has_moderate_members: true,
        ..Default::default()
    }));
    assert!(!can_finalize_review(&ModeratorFacts {
        has_mod_role: true,
        ..Default::default()
    }));
}

#[test]
fn can_open_discussion_matches_moderator() {
    assert!(!can_open_discussion(&ModeratorFacts::default()));
    assert!(can_open_discussion(&ModeratorFacts {
        has_mod_role: true,
        ..Default::default()
    }));
}

// ── finalize_sanction_plan (machine à états de finalisation) ──

#[test]
fn finalize_plan_non_loggable_actions() {
    for a in ["delete", "ignore", "unknown", ""] {
        assert_eq!(
            finalize_sanction_plan(a, false),
            FinalizeSanctionPlan::Nothing
        );
        assert_eq!(
            finalize_sanction_plan(a, true),
            FinalizeSanctionPlan::Nothing
        );
    }
}

#[test]
fn finalize_plan_nominal_with_strike() {
    for a in ["prevention", "warn", "mute", "ban"] {
        assert_eq!(
            finalize_sanction_plan(a, false),
            FinalizeSanctionPlan::LogWithStrike
        );
    }
}

#[test]
fn finalize_plan_anti_double_strike() {
    // Sévérité <= mute auto : pas de re-journalisation.
    for a in ["prevention", "warn", "mute"] {
        assert_eq!(
            finalize_sanction_plan(a, true),
            FinalizeSanctionPlan::AlreadyLogged
        );
    }
}

#[test]
fn finalize_plan_escalation_without_strike() {
    // BUG #5 : ban plus sévère que le mute auto -> journalisé sans strike.
    assert_eq!(
        finalize_sanction_plan("ban", true),
        FinalizeSanctionPlan::LogWithoutStrike
    );
}
