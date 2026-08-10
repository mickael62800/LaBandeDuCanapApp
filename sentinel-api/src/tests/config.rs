use super::*;

fn cfg(host: &str, port: u16, grpc_port: u16) -> AppConfig {
    AppConfig {
        database_url: String::new(),
        redis_url: String::new(),
        api_key: String::new(),
        host: host.into(),
        port,
        grpc_port,
        rate_limit_per_sec: 100,
        max_body_size: 1024,
        shutdown_timeout_secs: 30,
        allowed_origins: String::new(),
        metrics_token: String::new(),
        // Vide = verrou mono-serveur desactive : les tests ne doivent pas
        // dependre d'un identifiant de serveur particulier.
        guild_id: String::new(),
        nexus_api_url: String::new(),
        nexus_api_key: String::new(),
        docker_agent_token: String::new(),
        docker_agent_url: String::new(),
        discord_bot_token: String::new(),
        superadmin_user_ids: vec![],
        discord_oauth_client_id: String::new(),
        discord_oauth_client_secret: String::new(),
        discord_oauth_redirect_uri: String::new(),
        web_front_url: String::new(),
    }
}

#[test]
fn bind_addr_formats_host_port() {
    assert_eq!(cfg("127.0.0.1", 3000, 50051).bind_addr(), "127.0.0.1:3000");
    assert_eq!(cfg("0.0.0.0", 8080, 50051).bind_addr(), "0.0.0.0:8080");
}

#[test]
fn grpc_bind_addr_uses_grpc_port() {
    let c = cfg("0.0.0.0", 3000, 50051);
    assert_eq!(c.grpc_bind_addr(), "0.0.0.0:50051");
    assert_ne!(c.grpc_bind_addr(), c.bind_addr());
}

#[test]
fn bind_addr_handles_ipv6_style_host() {
    // [::1] reste tel quel dans le format actuel (pas de wrapping auto).
    assert_eq!(cfg("[::1]", 3000, 50051).bind_addr(), "[::1]:3000");
}

#[test]
fn bind_addrs_differ_when_ports_differ() {
    let c = cfg("0.0.0.0", 3000, 50051);
    assert_ne!(c.port, c.grpc_port);
    assert_ne!(c.bind_addr(), c.grpc_bind_addr());
}
