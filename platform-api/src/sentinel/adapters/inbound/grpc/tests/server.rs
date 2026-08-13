use super::*;

fn req_with_auth(value: Option<&str>) -> Request<()> {
    let mut req = Request::new(());
    if let Some(v) = value {
        req.metadata_mut()
            .insert("authorization", v.parse().unwrap());
    }
    req
}

#[test]
fn empty_api_key_disables_auth_and_passes_through() {
    let interceptor = build_auth_interceptor(String::new()).unwrap();
    // Sans header
    assert!(interceptor(req_with_auth(None)).is_ok());
    // Avec header arbitraire
    assert!(interceptor(req_with_auth(Some("Bearer whatever"))).is_ok());
}

#[test]
fn correct_bearer_token_is_accepted() {
    let interceptor = build_auth_interceptor("secret123".to_string()).unwrap();
    let req = req_with_auth(Some("Bearer secret123"));
    assert!(interceptor(req).is_ok());
}

#[test]
fn missing_token_is_unauthenticated() {
    let interceptor = build_auth_interceptor("secret123".to_string()).unwrap();
    let err = interceptor(req_with_auth(None)).unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[test]
fn wrong_token_is_unauthenticated() {
    let interceptor = build_auth_interceptor("secret123".to_string()).unwrap();
    let err = interceptor(req_with_auth(Some("Bearer wrong"))).unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[test]
fn token_without_bearer_prefix_is_unauthenticated() {
    let interceptor = build_auth_interceptor("secret123".to_string()).unwrap();
    let err = interceptor(req_with_auth(Some("secret123"))).unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[test]
fn missing_tls_configuration_allows_plain_http2() {
    assert!(build_server_builder(None).is_ok());
}

#[test]
fn configured_but_unreadable_tls_directory_is_rejected() {
    let missing =
        std::env::temp_dir().join(format!("sentinel-missing-grpc-tls-{}", std::process::id()));

    let error = build_server_builder(Some(&missing)).unwrap_err();

    assert!(error.contains("impossible de lire la configuration mTLS"));
    assert!(error.contains(&missing.display().to_string()));
}

#[test]
fn configured_but_invalid_tls_certificates_are_rejected() {
    let dir = std::env::temp_dir().join(format!(
        "sentinel-invalid-grpc-tls-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    for file in ["server.pem", "server.key", "ca.pem"] {
        std::fs::write(dir.join(file), b"not a PEM certificate").unwrap();
    }

    let error = build_server_builder(Some(&dir)).unwrap_err();

    assert!(error.contains("configuration mTLS invalide"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn invalid_api_key_chars_prevent_server_startup() {
    let error = build_auth_interceptor("bad\nkey\0".to_string())
        .err()
        .unwrap();
    assert!(error.contains("caracteres invalides"));
}
