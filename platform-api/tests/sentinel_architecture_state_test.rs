//! Garde-fous de dependances des handlers HTTP.

use std::path::Path;

fn rust_sources(root: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(root).expect("lecture du dossier handlers") {
        let path = entry.expect("entree handlers").path();
        if path.is_dir() {
            rust_sources(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn handlers_do_not_depend_on_app_state_or_execute_sql() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sentinel/adapters/inbound/http/handlers");
    let mut files = Vec::new();
    rust_sources(&root, &mut files);

    for file in files {
        let source = std::fs::read_to_string(&file).expect("lecture source handler");
        assert!(
            !source.contains("State<AppState>"),
            "{} extrait encore AppState",
            file.display()
        );
        assert!(
            !source.contains("sqlx::query"),
            "{} execute du SQL directement",
            file.display()
        );
        assert!(
            !source.contains("state.pg_pool"),
            "{} accede directement au pool PostgreSQL",
            file.display()
        );
    }
}

#[test]
fn app_state_is_only_an_aggregate_of_substates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(root.join("src/sentinel/bootstrap/state.rs"))
        .expect("lecture de la composition root");

    for forbidden in [
        "pub pg_pool:",
        "pub log_repo:",
        "pub bot_config_repo:",
        "pub discord_api:",
        "pub redis_client:",
        "pub job_client:",
    ] {
        assert!(
            !source.contains(forbidden),
            "AppState contient encore un champ plat historique: {forbidden}"
        );
    }
    assert!(source.contains("pub shared: SharedState"));
}

#[test]
fn audit_log_migrations_cover_the_verified_access_paths() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let query_indexes =
        std::fs::read_to_string(root.join("migrations/sentinel/032_audit_logs_query_indexes.sql"))
            .expect("lecture migration index audit_logs");
    let discord_index = std::fs::read_to_string(
        root.join("migrations/sentinel/031_discord_audit_sync_idempotency.sql"),
    )
    .expect("lecture migration index Discord");

    for expected in [
        "(guild_id, target_id, created_at DESC)",
        "(guild_id, actor_id, created_at DESC)",
        "(channel_id, event_type, created_at ASC)",
    ] {
        assert!(query_indexes.contains(expected), "index absent: {expected}");
    }
    assert!(discord_index.contains("(discord_entry_id, created_at)"));
}
