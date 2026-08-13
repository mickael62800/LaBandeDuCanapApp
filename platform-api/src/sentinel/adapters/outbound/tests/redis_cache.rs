use super::*;

#[test]
fn cache_stats_initial() {
    let stats = CacheStats {
        hits: 0,
        misses: 0,
        total: 0,
        hit_rate_percent: 0.0,
    };
    assert_eq!(stats.hit_rate_percent, 0.0);
}

#[test]
fn cache_stats_computation() {
    let stats = CacheStats {
        hits: 80,
        misses: 20,
        total: 100,
        hit_rate_percent: 80.0,
    };
    assert_eq!(stats.hits, 80);
    assert_eq!(stats.misses, 20);
    assert_eq!(stats.hit_rate_percent, 80.0);
}

#[test]
fn cache_stats_serializes() {
    let stats = CacheStats {
        hits: 42,
        misses: 8,
        total: 50,
        hit_rate_percent: 84.0,
    };
    let json = serde_json::to_string(&stats).unwrap();
    assert!(json.contains("\"hits\":42"));
    assert!(json.contains("\"hit_rate_percent\":84.0"));
}
