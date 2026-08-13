use super::*;

#[test]
fn is_configured_true_for_non_empty_token() {
    let svc = DiscordApiService::new("abc123".into());
    assert!(svc.is_configured());
}

#[test]
fn is_configured_false_for_empty_token() {
    let svc = DiscordApiService::new(String::new());
    assert!(!svc.is_configured());
}

#[test]
fn ensure_configured_returns_internal_error_when_empty() {
    let svc = DiscordApiService::new(String::new());
    let err = svc.ensure_configured().unwrap_err();
    match err {
        DomainError::Internal(msg) => assert!(msg.contains("SENTINEL_DISCORD_TOKEN")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn ensure_configured_ok_when_token_set() {
    let svc = DiscordApiService::new("t".into());
    assert!(svc.ensure_configured().is_ok());
}

#[test]
fn avatar_url_some_when_hash_provided() {
    let url = discord_avatar_url("1234567890", Some("abc")).unwrap();
    assert_eq!(
        url,
        "https://cdn.discordapp.com/avatars/1234567890/abc.png?size=64"
    );
}

#[test]
fn avatar_url_none_when_hash_missing() {
    assert!(discord_avatar_url("123", None).is_none());
}

#[test]
fn avatar_url_handles_animated_hash() {
    // Discord utilise le prefixe "a_" pour les GIF animes — helper le conserve tel quel.
    let url = discord_avatar_url("42", Some("a_deadbeef")).unwrap();
    assert!(url.contains("/42/a_deadbeef.png"));
}

#[test]
fn user_guild_deserializes_only_id() {
    let raw = serde_json::json!({"id": "g1", "name": "My Guild", "icon": null, "permissions": "0"});
    let g: UserGuild = serde_json::from_value(raw).unwrap();
    assert_eq!(g.id, "g1");
}

#[test]
fn discord_user_default_avatar_absent() {
    let raw = serde_json::json!({"id": "u", "username": "alice"});
    let u: DiscordUser = serde_json::from_value(raw).unwrap();
    assert_eq!(u.id, "u");
    assert_eq!(u.username, "alice");
    assert!(u.avatar.is_none());
}

#[test]
fn discord_user_with_avatar() {
    let raw = serde_json::json!({"id": "u", "username": "alice", "avatar": "hash"});
    let u: DiscordUser = serde_json::from_value(raw).unwrap();
    assert_eq!(u.avatar.as_deref(), Some("hash"));
}

#[test]
fn discord_channel_deserializes_required_fields() {
    let raw = serde_json::json!({"id": "c1", "name": "general", "position": 3});
    let c: DiscordChannel = serde_json::from_value(raw).unwrap();
    assert_eq!(c.id, "c1");
    assert_eq!(c.name, "general");
    assert_eq!(c.position, 3);
}

#[test]
fn discord_member_roundtrip_json() {
    let m = DiscordMember {
        id: "u".into(),
        username: "alice".into(),
        display_name: Some("Alice".into()),
        avatar_url: Some("https://example/x.png".into()),
    };
    let json = serde_json::to_value(&m).unwrap();
    let back: DiscordMember = serde_json::from_value(json).unwrap();
    assert_eq!(back.id, "u");
    assert_eq!(back.display_name.as_deref(), Some("Alice"));
}

// ── Toutes les methodes DiscordApi retournent Internal error avec token vide ──
//
// Chaque methode appelle `ensure_configured()` en premier : avec un token vide,
// elle doit retourner DomainError::Internal sans tenter d'appel HTTP.

use crate::sentinel::adapters::outbound::discord_api::DiscordApi;

async fn unconfigured_service() -> DiscordApiService {
    DiscordApiService::new(String::new())
}

#[tokio::test]
async fn list_text_channels_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc.list_text_channels("g").await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn upload_emoji_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc
        .upload_emoji("g", "name", &[0u8; 10], "image/png")
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn ban_user_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc.ban_user("g", "u", "reason").await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn list_members_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc.list_members("g", 100).await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn send_dm_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc.send_dm("u", "hello").await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn create_role_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc
        .create_role("g", "Role", 0xFF0000, None)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn edit_role_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc
        .edit_role("g", "r", Some("New"), Some(0), None, None, None)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn delete_role_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc.delete_role("g", "r").await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn unban_user_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc.unban_user("g", "u").await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn remove_timeout_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc.remove_timeout("g", "u").await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn apply_timeout_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc.apply_timeout("g", "u", 600).await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn get_user_guilds_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc.get_user_guilds("access_token").await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

#[tokio::test]
async fn get_user_me_unconfigured_returns_internal_error() {
    let svc = unconfigured_service().await;
    let err = svc.get_user_me("access_token").await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}

// ── Sanity checks sur les DTOs et options de serde ──

#[test]
fn discord_user_with_null_avatar_in_json() {
    let raw = serde_json::json!({"id": "u", "username": "alice", "avatar": null});
    let u: DiscordUser = serde_json::from_value(raw).unwrap();
    assert!(u.avatar.is_none());
}

#[test]
fn discord_channel_serializes_roundtrip() {
    let c = DiscordChannel {
        id: "c".into(),
        name: "general".into(),
        position: 5,
        kind: "text".to_string(),
    };
    let json = serde_json::to_value(&c).unwrap();
    let back: DiscordChannel = serde_json::from_value(json).unwrap();
    assert_eq!(back.position, 5);
}

#[test]
fn avatar_url_includes_size_64() {
    let url = discord_avatar_url("123", Some("hash")).unwrap();
    assert!(url.contains("size=64"));
}

#[test]
fn avatar_url_uses_png_extension() {
    let url = discord_avatar_url("123", Some("hash")).unwrap();
    assert!(url.contains(".png?"));
}
