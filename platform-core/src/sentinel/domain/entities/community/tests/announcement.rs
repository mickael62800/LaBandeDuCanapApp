use super::super::announcement::*;
use chrono::{TimeZone, Utc};

fn dt(y: i32, m: u32, d: u32, h: u32, mn: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, h, mn, 0).unwrap()
}

// ── compute_next_run_at : Once ─────────────────────────────────────────

#[test]
fn once_returns_scheduled_at_if_future() {
    let target = dt(2026, 6, 1, 14, 0);
    let now = dt(2026, 5, 1, 10, 0);
    let next = compute_next_run_at(
        RecurrenceType::Once,
        14,
        0,
        None,
        None,
        None,
        Some(target),
        None,
        now,
    );
    assert_eq!(next, Some(target));
}

#[test]
fn once_returns_none_if_past() {
    let target = dt(2026, 4, 1, 14, 0);
    let now = dt(2026, 5, 1, 10, 0);
    let next = compute_next_run_at(
        RecurrenceType::Once,
        14,
        0,
        None,
        None,
        None,
        Some(target),
        None,
        now,
    );
    assert_eq!(next, None);
}

// ── compute_next_run_at : Daily ────────────────────────────────────────

#[test]
fn daily_today_if_hour_not_passed() {
    let now = dt(2026, 5, 2, 10, 0);
    let next = compute_next_run_at(
        RecurrenceType::Daily,
        14,
        0,
        None,
        None,
        None,
        None,
        None,
        now,
    );
    assert_eq!(next, Some(dt(2026, 5, 2, 14, 0)));
}

#[test]
fn daily_tomorrow_if_hour_passed() {
    let now = dt(2026, 5, 2, 16, 0);
    let next = compute_next_run_at(
        RecurrenceType::Daily,
        14,
        0,
        None,
        None,
        None,
        None,
        None,
        now,
    );
    assert_eq!(next, Some(dt(2026, 5, 3, 14, 0)));
}

#[test]
fn daily_with_minute() {
    let now = dt(2026, 5, 2, 14, 30);
    let next = compute_next_run_at(
        RecurrenceType::Daily,
        14,
        45,
        None,
        None,
        None,
        None,
        None,
        now,
    );
    assert_eq!(next, Some(dt(2026, 5, 2, 14, 45)));
}

#[test]
fn daily_minute_passed() {
    let now = dt(2026, 5, 2, 14, 45);
    let next = compute_next_run_at(
        RecurrenceType::Daily,
        14,
        30,
        None,
        None,
        None,
        None,
        None,
        now,
    );
    // 14h30 deja passe -> demain
    assert_eq!(next, Some(dt(2026, 5, 3, 14, 30)));
}

// ── compute_next_run_at : Weekly ───────────────────────────────────────

#[test]
fn weekly_same_day_if_hour_not_passed() {
    // Vendredi 2 mai 2026 a 10h, target vendredi 14h -> aujourd'hui 14h
    // Note : 2026-05-02 = Samedi (a verifier). Mais peu importe, on
    // teste que si dow_today == dow_target ET hour pas passee, c'est
    // aujourd'hui.
    let now = dt(2026, 5, 4, 10, 0); // 2026-05-04 = lundi
                                     // day_of_week 0 = Lundi
    let next = compute_next_run_at(
        RecurrenceType::Weekly,
        14,
        0,
        Some(0),
        None,
        None,
        None,
        None,
        now,
    );
    assert_eq!(next, Some(dt(2026, 5, 4, 14, 0)));
}

#[test]
fn weekly_next_week_if_same_day_hour_passed() {
    let now = dt(2026, 5, 4, 16, 0); // lundi 16h
    let next = compute_next_run_at(
        RecurrenceType::Weekly,
        14,
        0,
        Some(0),
        None,
        None,
        None,
        None,
        now,
    );
    // -> lundi prochain
    assert_eq!(next, Some(dt(2026, 5, 11, 14, 0)));
}

#[test]
fn weekly_advances_to_target_dow() {
    // Lundi 4 mai 2026, target = vendredi (4)
    let now = dt(2026, 5, 4, 10, 0);
    let next = compute_next_run_at(
        RecurrenceType::Weekly,
        14,
        0,
        Some(4),
        None,
        None,
        None,
        None,
        now,
    );
    // Vendredi 8 mai
    assert_eq!(next, Some(dt(2026, 5, 8, 14, 0)));
}

#[test]
fn weekly_wraps_to_next_week() {
    // Vendredi 8 mai, target = lundi (0)
    let now = dt(2026, 5, 8, 10, 0); // vendredi
    let next = compute_next_run_at(
        RecurrenceType::Weekly,
        14,
        0,
        Some(0),
        None,
        None,
        None,
        None,
        now,
    );
    // Lundi 11 mai
    assert_eq!(next, Some(dt(2026, 5, 11, 14, 0)));
}

// ── compute_next_run_at : Monthly ──────────────────────────────────────

#[test]
fn monthly_same_month_if_day_not_passed() {
    let now = dt(2026, 5, 2, 10, 0);
    let next = compute_next_run_at(
        RecurrenceType::Monthly,
        14,
        0,
        None,
        Some(15),
        None,
        None,
        None,
        now,
    );
    assert_eq!(next, Some(dt(2026, 5, 15, 14, 0)));
}

#[test]
fn monthly_next_month_if_day_passed() {
    let now = dt(2026, 5, 20, 10, 0);
    let next = compute_next_run_at(
        RecurrenceType::Monthly,
        14,
        0,
        None,
        Some(15),
        None,
        None,
        None,
        now,
    );
    assert_eq!(next, Some(dt(2026, 6, 15, 14, 0)));
}

#[test]
fn monthly_31_in_february_clamps_to_28() {
    // 1er fevrier 2026, target jour 31 -> 28 fevrier (annee non bissextile)
    let now = dt(2026, 2, 1, 10, 0);
    let next = compute_next_run_at(
        RecurrenceType::Monthly,
        14,
        0,
        None,
        Some(31),
        None,
        None,
        None,
        now,
    );
    assert_eq!(next, Some(dt(2026, 2, 28, 14, 0)));
}

#[test]
fn monthly_31_in_february_leap_year_clamps_to_29() {
    let now = dt(2024, 2, 1, 10, 0);
    let next = compute_next_run_at(
        RecurrenceType::Monthly,
        14,
        0,
        None,
        Some(31),
        None,
        None,
        None,
        now,
    );
    assert_eq!(next, Some(dt(2024, 2, 29, 14, 0)));
}

#[test]
fn monthly_year_wrap() {
    let now = dt(2026, 12, 20, 10, 0);
    let next = compute_next_run_at(
        RecurrenceType::Monthly,
        14,
        0,
        None,
        Some(15),
        None,
        None,
        None,
        now,
    );
    assert_eq!(next, Some(dt(2027, 1, 15, 14, 0)));
}

// ── end_date ───────────────────────────────────────────────────────────

#[test]
fn end_date_blocks_future_runs() {
    let now = dt(2026, 5, 2, 10, 0);
    let end = dt(2026, 5, 2, 12, 0); // end avant la prochaine occurrence (14h)
    let next = compute_next_run_at(
        RecurrenceType::Daily,
        14,
        0,
        None,
        None,
        None,
        None,
        Some(end),
        now,
    );
    assert_eq!(next, None);
}

#[test]
fn end_date_allows_runs_before_end() {
    let now = dt(2026, 5, 2, 10, 0);
    let end = dt(2026, 5, 5, 23, 59);
    let next = compute_next_run_at(
        RecurrenceType::Daily,
        14,
        0,
        None,
        None,
        None,
        None,
        Some(end),
        now,
    );
    assert_eq!(next, Some(dt(2026, 5, 2, 14, 0)));
}

// ── render_template : interpolation des variables ──────────────────────

#[test]
fn render_replaces_basic_variables() {
    let ctx = InterpolationContext {
        now: dt(2026, 5, 2, 14, 30),
        guild_name: "Mon Discord",
    };
    let out = render_template("Le {date} a {time} sur {guild_name}", &ctx);
    assert_eq!(out, "Le 2026-05-02 a 14:30 sur Mon Discord");
}

#[test]
fn render_replaces_day_and_month_names() {
    // 2026-05-04 = lundi, mai
    let ctx = InterpolationContext {
        now: dt(2026, 5, 4, 10, 0),
        guild_name: "X",
    };
    let out = render_template("{day_name} {day} {month_name} {year}", &ctx);
    assert_eq!(out, "lundi 04 mai 2026");
}

#[test]
fn render_unknown_variable_left_as_is() {
    let ctx = InterpolationContext {
        now: dt(2026, 5, 2, 14, 0),
        guild_name: "X",
    };
    let out = render_template("hello {unknown} world", &ctx);
    assert_eq!(out, "hello {unknown} world");
}

#[test]
fn render_no_variables_returns_template() {
    let ctx = InterpolationContext {
        now: dt(2026, 5, 2, 14, 0),
        guild_name: "X",
    };
    let out = render_template("hello world", &ctx);
    assert_eq!(out, "hello world");
}

#[test]
fn render_multiple_occurrences_of_same_variable() {
    let ctx = InterpolationContext {
        now: dt(2026, 5, 2, 14, 0),
        guild_name: "X",
    };
    let out = render_template("{date} - {date}", &ctx);
    assert_eq!(out, "2026-05-02 - 2026-05-02");
}

// ── Conversions enum string ────────────────────────────────────────────

#[test]
fn recurrence_type_roundtrip() {
    for t in [
        RecurrenceType::Once,
        RecurrenceType::Daily,
        RecurrenceType::Weekly,
        RecurrenceType::Monthly,
    ] {
        assert_eq!(RecurrenceType::from_str(t.as_str()), Some(t));
    }
}

#[test]
fn content_type_roundtrip() {
    for t in [ContentType::Text, ContentType::Embed] {
        assert_eq!(ContentType::from_str(t.as_str()), Some(t));
    }
}

#[test]
fn run_status_roundtrip() {
    for t in [
        RunStatus::Pending,
        RunStatus::Success,
        RunStatus::Partial,
        RunStatus::Error,
    ] {
        assert_eq!(RunStatus::from_str(t.as_str()), Some(t));
    }
}
