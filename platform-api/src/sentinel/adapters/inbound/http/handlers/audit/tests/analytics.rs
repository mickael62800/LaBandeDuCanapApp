use super::*;

#[test]
fn cache_key_without_guild_uses_global() {
    let k = cache_key("top_users", None, 30, None);
    assert_eq!(k, "analytics:top_users:global:30");
}

#[test]
fn cache_key_with_guild_and_limit() {
    let k = cache_key("top_users", Some("123456789012345678"), 7, Some(20));
    assert_eq!(k, "analytics:top_users:123456789012345678:7:20");
}

#[test]
fn cache_key_with_guild_no_limit() {
    let k = cache_key("heatmap", Some("g"), 14, None);
    assert_eq!(k, "analytics:heatmap:g:14");
}

#[test]
fn cache_key_uses_endpoint_verbatim() {
    let k = cache_key("some-endpoint.with.dots", None, 1, Some(1));
    assert!(k.contains("some-endpoint.with.dots"));
}
