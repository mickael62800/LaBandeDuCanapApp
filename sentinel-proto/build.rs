fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Strategie protoc :
    // 1. Si PROTOC est deja defini dans l'env, on respecte (override explicite).
    // 2. Sinon, on tente d'utiliser le binaire systeme (Linux Alpine/Debian
    //    via apk/apt — necessaire car les binaires prebuilt sont glibc-linked
    //    et ne tournent PAS sur Alpine musl).
    // 3. Sinon (Windows/macOS dev sans protoc installe), on retombe sur le
    //    binaire prebuilt embarque via `protoc-bin-vendored`.
    let has_system_protoc = std::env::var("PROTOC").is_ok()
        || std::process::Command::new("protoc")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    if !has_system_protoc {
        let protoc = protoc_bin_vendored::protoc_bin_path()?;
        std::env::set_var("PROTOC", protoc);
    }

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "proto/age_gate.proto",
                "proto/ai_dataset.proto",
                "proto/announcements.proto",
                "proto/audit.proto",
                "proto/automod.proto",
                "proto/automod_review.proto",
                "proto/common.proto",
                "proto/community.proto",
                "proto/confessions.proto",
                "proto/discord_messages.proto",
                "proto/embeds.proto",
                "proto/export.proto",
                "proto/guild_backup.proto",
                "proto/ideas.proto",
                "proto/images.proto",
                "proto/members.proto",
                "proto/moderation.proto",
                "proto/progression.proto",
                "proto/purge.proto",
                "proto/roles.proto",
                "proto/security.proto",
                "proto/security_state.proto",
                "proto/stats.proto",
                "proto/sursis.proto",
                "proto/tickets.proto",
                "proto/voice.proto",
                "proto/welcome.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}
