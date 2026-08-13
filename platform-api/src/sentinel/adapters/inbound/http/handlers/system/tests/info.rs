use super::*;

// Les tests de `parse_redis_info` vivent avec la fonction dans
// `adapters/outbound/system/host_metrics.rs`.

#[test]
fn uptime_seconds_initializes_and_returns_value() {
    // Le premier appel initialise STARTED_AT ; les suivants doivent reutiliser.
    let a = uptime_seconds();
    let b = uptime_seconds();
    assert!(b >= a);
}

#[test]
fn record_startup_is_idempotent() {
    // OnceLock → le 2e set est ignore silencieusement.
    record_startup();
    record_startup();
    // Pas de panique attendue.
}
