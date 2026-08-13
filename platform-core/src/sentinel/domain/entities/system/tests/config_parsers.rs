use super::*;

fn make_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// ── parse_bool_config ──

#[test]
fn parse_bool_missing_key_returns_default() {
    let m = make_map(&[]);
    assert!(parse_bool_config(&m, "flag", true));
    assert!(!parse_bool_config(&m, "flag", false));
}

#[test]
fn parse_bool_accepts_true_1_yes_case_insensitive() {
    for v in ["true", "TRUE", "True", "1", "yes", "YES", "Yes"] {
        let m = make_map(&[("f", v)]);
        assert!(parse_bool_config(&m, "f", false), "should be true for {v}");
    }
}

#[test]
fn parse_bool_rejects_other_strings() {
    for v in ["false", "0", "no", "nope", "", "2"] {
        let m = make_map(&[("f", v)]);
        assert!(
            !parse_bool_config(&m, "f", true),
            "should be false for '{v}'"
        );
    }
}

// ── parse_i64_config ──

#[test]
fn parse_i64_missing_key_returns_default() {
    let m = make_map(&[]);
    assert_eq!(parse_i64_config(&m, "n", 42), 42);
    assert_eq!(parse_i64_config(&m, "n", -1), -1);
}

#[test]
fn parse_i64_parses_positive_integer() {
    let m = make_map(&[("n", "10000")]);
    assert_eq!(parse_i64_config(&m, "n", 0), 10000);
}

#[test]
fn parse_i64_parses_negative_integer() {
    let m = make_map(&[("n", "-123")]);
    assert_eq!(parse_i64_config(&m, "n", 0), -123);
}

#[test]
fn parse_i64_invalid_value_returns_default() {
    for v in ["abc", "", "1.5", "12a", " 10"] {
        let m = make_map(&[("n", v)]);
        assert_eq!(parse_i64_config(&m, "n", 99), 99, "default for '{v}'");
    }
}

// ── is_worker_service ──

#[test]
fn is_worker_true_when_name_contains_worker() {
    assert!(is_worker_service("moderation-worker"));
    assert!(is_worker_service("audit-worker"));
    assert!(is_worker_service("worker-common"));
}

#[test]
fn is_worker_false_for_bot_names() {
    assert!(!is_worker_service("audit-bot"));
    assert!(!is_worker_service("moderation-bot"));
    assert!(!is_worker_service("game-bot"));
    assert!(!is_worker_service(""));
}

// ── parsers de lignes (pipe / id:u64 / csv) ──

#[test]
fn pipe_simple() {
    let r = parse_pipe_lines("A|B\nC|D");
    assert_eq!(r, vec![("A".into(), "B".into()), ("C".into(), "D".into())]);
}

#[test]
fn pipe_ignores_empty() {
    assert_eq!(parse_pipe_lines("\n\nA|B\n\n").len(), 1);
}

#[test]
fn pipe_ignores_invalid() {
    assert_eq!(parse_pipe_lines("no sep\n|b\na|\nOK|V").len(), 1);
}

#[test]
fn pipe_trims() {
    let r = parse_pipe_lines("  X  |  Y  ");
    assert_eq!(r[0], ("X".into(), "Y".into()));
}

#[test]
fn id_u64_simple() {
    let r = parse_id_u64_lines("111:3600\n222:86400");
    assert_eq!(r, vec![(111, 3600), (222, 86400)]);
}

#[test]
fn csv_simple() {
    let r = split_csv("a, B , c");
    assert_eq!(r, vec!["a", "b", "c"]);
}

#[test]
fn csv_empty() {
    assert!(split_csv("").is_empty());
}

#[test]
fn lookup_u64_found() {
    assert_eq!(lookup_u64(&[(1, 100)], 1), Some(100));
}

#[test]
fn lookup_u64_none() {
    assert_eq!(lookup_u64(&[(1, 100)], 99), None);
}

#[test]
fn default_log_category_by_name() {
    assert_eq!(default_log_category("sentinel-worker"), "worker");
    assert_eq!(default_log_category("automod-bot"), "bot");
    assert_eq!(default_log_category("dashboard"), "discord");
}

#[test]
fn enabled_flag_absent_is_disabled() {
    assert!(!parse_enabled_flag(None));
}

#[test]
fn enabled_flag_parses_value() {
    assert!(parse_enabled_flag(Some("true")));
    assert!(parse_enabled_flag(Some("1")));
    assert!(!parse_enabled_flag(Some("false")));
    assert!(!parse_enabled_flag(Some("0")));
    assert!(!parse_enabled_flag(Some("no")));
}

#[test]
fn u64_csv_ignores_invalid() {
    assert_eq!(
        parse_u64_csv("300, 3600 ,abc,86400"),
        vec![300, 3600, 86400]
    );
    assert!(parse_u64_csv("").is_empty());
}
