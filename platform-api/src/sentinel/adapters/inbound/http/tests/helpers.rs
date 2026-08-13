use super::*;

// ── normalize_limit ─────────────────────────────────────

#[test]
fn normalize_limit_none_uses_default() {
    assert_eq!(normalize_limit(None, 50, 1000), 50);
}

#[test]
fn normalize_limit_caps_at_max() {
    assert_eq!(normalize_limit(Some(9999), 50, 100), 100);
}

#[test]
fn normalize_limit_floors_negative_to_zero() {
    assert_eq!(normalize_limit(Some(-5), 50, 100), 0);
}

#[test]
fn normalize_limit_preserves_valid() {
    assert_eq!(normalize_limit(Some(25), 50, 100), 25);
}

#[test]
fn normalize_limit_edge_at_max() {
    assert_eq!(normalize_limit(Some(100), 50, 100), 100);
}

// ── normalize_days ──────────────────────────────────────

#[test]
fn normalize_days_none_uses_default() {
    assert_eq!(normalize_days(None, 7, 90), 7);
}

#[test]
fn normalize_days_zero_floors_to_one() {
    assert_eq!(normalize_days(Some(0), 7, 90), 1);
}

#[test]
fn normalize_days_negative_floors_to_one() {
    assert_eq!(normalize_days(Some(-10), 7, 90), 1);
}

#[test]
fn normalize_days_caps_at_max() {
    assert_eq!(normalize_days(Some(365), 7, 90), 90);
}

#[test]
fn normalize_days_valid_preserved() {
    assert_eq!(normalize_days(Some(30), 7, 90), 30);
}

// ── normalize_offset ────────────────────────────────────

#[test]
fn normalize_offset_none_is_zero() {
    assert_eq!(normalize_offset(None), 0);
}

#[test]
fn normalize_offset_negative_is_zero() {
    assert_eq!(normalize_offset(Some(-100)), 0);
}

#[test]
fn normalize_offset_preserves_positive() {
    assert_eq!(normalize_offset(Some(42)), 42);
}

// ── ok_response ─────────────────────────────────────────

#[test]
fn ok_response_returns_ok_true() {
    let Json(v) = ok_response();
    assert_eq!(v["ok"], true);
}

// ── map_to_dtos / single_dto ────────────────────────────

struct Src(i32);
#[derive(serde::Serialize)]
struct Dst(i32);
impl From<Src> for Dst {
    fn from(s: Src) -> Self {
        Dst(s.0 * 2)
    }
}

#[test]
fn map_to_dtos_applies_conversion() {
    let Json(out) = map_to_dtos::<Src, Dst>(vec![Src(1), Src(2), Src(3)]);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].0, 2);
    assert_eq!(out[2].0, 6);
}

#[test]
fn map_to_dtos_empty_input_returns_empty() {
    let Json(out) = map_to_dtos::<Src, Dst>(vec![]);
    assert!(out.is_empty());
}

#[test]
fn single_dto_wraps_conversion() {
    let Json(out) = single_dto::<Src, Dst>(Src(5));
    assert_eq!(out.0, 10);
}
