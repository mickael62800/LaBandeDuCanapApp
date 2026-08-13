use super::*;

use chrono::Utc;
use uuid::Uuid;

fn make_channel() -> VoiceChannel {
    VoiceChannel {
        id: Uuid::new_v4(),
        guild_id: "guild1".into(),
        owner_id: "owner1".into(),
        owner_name: "Owner".into(),
        channel_id: "chan1".into(),
        text_channel_id: Some("text1".into()),
        members_channel_id: Some("mem1".into()),
        queue_channel_id: None,
        category_id: Some("cat1".into()),
        channel_name: "Salon de Owner".into(),
        kind:
            platform_core::sentinel::domain::enums::community::voice_channel_kind::VoiceChannelKind::Private,
        visibility: "visible".into(),
        queue_enabled: false,
        locked: false,
        stage_enabled: false,
        member_limit: Some(10),
        status: Some("Cool".into()),
        channel_status: "open".into(),
        closed_at: None,
        created_at: Utc::now(),
    }
}

fn make_theme() -> VoiceChannelTheme {
    VoiceChannelTheme {
        id: Uuid::new_v4(),
        guild_id: "guild1".into(),
        name: "Gaming".into(),
        emoji: Some("🎮".into()),
        channel_name_template: "{user} Gaming".into(),
        member_limit: Some(5),
        visibility: "visible".into(),
        locked: false,
        queue_enabled: false,
        bitrate: Some(64000),
        slowmode_secs: Some(10),
        stage_enabled: true,
        is_default: true,
        sort_order: 0,
        created_at: Utc::now(),
    }
}

fn make_invite_link() -> VoiceChannelInviteLink {
    VoiceChannelInviteLink {
        id: Uuid::new_v4(),
        voice_channel_id: Uuid::new_v4(),
        guild_id: "guild1".into(),
        channel_id: "chan1".into(),
        created_by: "user1".into(),
        created_by_name: "User".into(),
        code: "ABCD1234".into(),
        max_uses: Some(5),
        current_uses: 2,
        expires_at: Utc::now() + chrono::Duration::hours(1),
        revoked: false,
        created_at: Utc::now(),
    }
}

// ── VoiceChannel → VoiceChannelResponseDto ──

#[test]
fn channel_to_dto_preserves_fields() {
    let ch = make_channel();
    let id = ch.id;
    let dto = VoiceChannelResponseDto::from(ch);
    assert_eq!(dto.id, id.to_string());
    assert_eq!(dto.guild_id, "guild1".into());
    assert_eq!(dto.kind, "private");
    assert_eq!(dto.member_limit, Some(10));
    assert!(!dto.stage_enabled);
}

#[test]
fn channel_to_dto_formats_dates() {
    let ch = make_channel();
    let dto = VoiceChannelResponseDto::from(ch);
    assert!(dto.created_at.contains("T")); // RFC3339 format
    assert!(dto.closed_at.is_none());
}

#[test]
fn channel_to_dto_closed_at_some() {
    let mut ch = make_channel();
    ch.closed_at = Some(Utc::now());
    let dto = VoiceChannelResponseDto::from(ch);
    assert!(dto.closed_at.is_some());
}

// ── VoiceChannelTheme → ThemeResponseDto ──

#[test]
fn theme_to_dto_preserves_all_fields() {
    let theme = make_theme();
    let dto = ThemeResponseDto::from(theme);
    assert_eq!(dto.name, "Gaming");
    assert_eq!(dto.emoji, Some("🎮".into()));
    assert_eq!(dto.member_limit, Some(5));
    assert_eq!(dto.bitrate, Some(64000));
    assert_eq!(dto.slowmode_secs, Some(10));
    assert!(dto.stage_enabled);
    assert!(dto.is_default);
}

#[test]
fn theme_to_dto_none_optionals() {
    let mut theme = make_theme();
    theme.emoji = None;
    theme.member_limit = None;
    theme.bitrate = None;
    theme.slowmode_secs = None;
    let dto = ThemeResponseDto::from(theme);
    assert!(dto.emoji.is_none());
    assert!(dto.member_limit.is_none());
}

// ── VoiceChannelInviteLink → InviteLinkResponseDto ──

#[test]
fn invite_link_to_dto_preserves_fields() {
    let link = make_invite_link();
    let dto = InviteLinkResponseDto::from(link);
    assert_eq!(dto.code, "ABCD1234");
    assert_eq!(dto.max_uses, Some(5));
    assert_eq!(dto.current_uses, 2);
    assert!(!dto.revoked);
}

#[test]
fn invite_link_to_dto_formats_dates() {
    let link = make_invite_link();
    let dto = InviteLinkResponseDto::from(link);
    assert!(dto.expires_at.contains("T"));
    assert!(dto.created_at.contains("T"));
}

// ── CreateThemeDto → CreateThemeCommand ──

#[test]
fn theme_dto_to_command_sets_empty_guild() {
    let dto = CreateThemeDto {
        name: "Test".into(),
        emoji: None,
        channel_name_template: "{user}".into(),
        member_limit: None,
        visibility: "visible".into(),
        locked: false,
        queue_enabled: false,
        bitrate: None,
        slowmode_secs: None,
        stage_enabled: false,
        is_default: false,
        sort_order: 0,
    };
    let cmd: CreateThemeCommand = dto.into();
    assert_eq!(cmd.guild_id, "".into()); // set by handler
    assert_eq!(cmd.name, "Test");
}

// ── VoiceChannelDetail → VoiceChannelDetailDto ──

#[test]
fn detail_to_dto_aggregates_all() {
    let detail = VoiceChannelDetail {
        channel: make_channel(),
        co_admins: vec![],
        bans: vec![],
        invite_links: vec![make_invite_link()],
    };
    let dto = VoiceChannelDetailDto::from(detail);
    assert!(dto.co_admins.is_empty());
    assert!(dto.bans.is_empty());
    assert_eq!(dto.invite_links.len(), 1);
    assert_eq!(dto.invite_links[0].code, "ABCD1234");
}

// ── Default functions ──

#[test]
fn default_kind_is_public() {
    assert_eq!(default_kind(), "public");
}

#[test]
fn default_visibility_is_visible() {
    assert_eq!(default_visibility(), "visible");
}

#[test]
fn default_channel_name_template_is_user() {
    assert_eq!(default_channel_name_template(), "{user}");
}

// ── CreateVoiceChannelDto deserialize + From ──

#[test]
fn create_voice_channel_dto_deserializes_with_defaults() {
    let dto: CreateVoiceChannelDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "owner_id": "o", "owner_name": "O",
        "channel_id": "c", "channel_name": "Salon"
    }))
    .unwrap();
    assert_eq!(dto.kind, "public");
    assert_eq!(dto.visibility, "visible");
    assert!(!dto.queue_enabled);
    assert!(!dto.stage_enabled);
}

#[test]
fn create_voice_channel_dto_to_command() {
    let dto = CreateVoiceChannelDto {
        guild_id: "g".into(),
        owner_id: "o".into(),
        owner_name: "Own".into(),
        channel_id: "c".into(),
        text_channel_id: Some("t".into()),
        members_channel_id: None,
        queue_channel_id: None,
        category_id: None,
        channel_name: "Salon".into(),
        kind: "private".into(),
        visibility: "hidden".into(),
        queue_enabled: true,
        stage_enabled: true,
    };
    let cmd: CreateVoiceChannelCommand = dto.into();
    assert_eq!(cmd.guild_id, "g".into());
    assert_eq!(cmd.kind, "private");
    assert!(cmd.queue_enabled);
    assert!(cmd.stage_enabled);
    assert_eq!(cmd.text_channel_id.as_deref(), Some("t"));
}

// ── UpdateVoiceChannelDto ──

#[test]
fn update_dto_all_optional() {
    let dto: UpdateVoiceChannelDto = serde_json::from_str("{}").unwrap();
    assert!(dto.visibility.is_none());
    assert!(dto.locked.is_none());
    assert!(dto.queue_enabled.is_none());
    assert!(dto.name.is_none());
    assert!(dto.status.is_none());
    assert!(dto.member_limit.is_none());
    assert!(dto.queue_channel_id.is_none());
    assert!(dto.stage_enabled.is_none());
}

#[test]
fn update_dto_member_limit_present() {
    let dto: UpdateVoiceChannelDto =
        serde_json::from_value(serde_json::json!({"member_limit": 10})).unwrap();
    assert_eq!(dto.member_limit, Some(Some(10)));
}

#[test]
fn update_dto_name_and_status_set() {
    let dto: UpdateVoiceChannelDto = serde_json::from_value(serde_json::json!({
        "name": "Nouveau nom",
        "status": "AFK"
    }))
    .unwrap();
    assert_eq!(dto.name.as_deref(), Some("Nouveau nom"));
    assert_eq!(dto.status.as_deref(), Some("AFK"));
}

// ── TransferOwnership / AddCoAdmin / AddWhitelist / BanFromChannel ──

#[test]
fn transfer_ownership_dto_deserializes() {
    let dto: TransferOwnershipDto =
        serde_json::from_value(serde_json::json!({"new_owner_id": "u1", "new_owner_name": "U1"}))
            .unwrap();
    assert_eq!(dto.new_owner_id, "u1");
}

#[test]
fn add_co_admin_dto_deserializes() {
    let dto: AddCoAdminDto =
        serde_json::from_value(serde_json::json!({"user_id": "u", "user_name": "U"})).unwrap();
    assert_eq!(dto.user_id, "u".into());
}

#[test]
fn add_whitelist_dto_deserializes() {
    let dto: AddWhitelistDto = serde_json::from_value(
        serde_json::json!({"guild_id": "g", "owner_id": "o", "target_id": "t", "target_name": "T"}),
    )
    .unwrap();
    assert_eq!(dto.target_name, "T");
}

#[test]
fn ban_from_channel_dto_with_reason_and_duration() {
    let dto: BanFromChannelDto = serde_json::from_value(serde_json::json!({
        "user_id": "u", "user_name": "U", "banned_by": "m",
        "reason": "spam", "duration_secs": 3600
    }))
    .unwrap();
    assert_eq!(dto.reason.as_deref(), Some("spam"));
    assert_eq!(dto.duration_secs, Some(3600));
}

#[test]
fn ban_from_channel_dto_permanent() {
    let dto: BanFromChannelDto = serde_json::from_value(serde_json::json!({
        "user_id": "u", "user_name": "U", "banned_by": "m"
    }))
    .unwrap();
    assert!(dto.reason.is_none());
    assert!(dto.duration_secs.is_none());
}

// ── CreateInviteLink / UseInviteLink ──

#[test]
fn create_invite_link_dto_defaults_and_to_command() {
    let dto = CreateInviteLinkDto {
        created_by: "u".into(),
        created_by_name: "U".into(),
        duration_secs: Some(1800),
        max_uses: Some(5),
    };
    let cmd: CreateInviteLinkCommand = dto.into();
    assert_eq!(cmd.channel_id, "".into()); // set by handler
    assert_eq!(cmd.duration_secs, Some(1800));
    assert_eq!(cmd.max_uses, Some(5));
}

#[test]
fn use_invite_link_dto_deserializes() {
    let dto: UseInviteLinkDto =
        serde_json::from_value(serde_json::json!({"user_id": "u", "user_name": "U"})).unwrap();
    assert_eq!(dto.user_id, "u".into());
}

// ── CoAdmin / Whitelist / Ban response DTOs ──

#[test]
fn co_admin_to_dto_preserves_fields() {
    let ca = VoiceChannelCoAdmin {
        id: Uuid::new_v4(),
        voice_channel_id: Uuid::new_v4(),
        user_id: "u".into(),
        user_name: "Name".into(),
        granted_at: Utc::now(),
    };
    let dto: CoAdminResponseDto = ca.into();
    assert_eq!(dto.user_id, "u".into());
    assert_eq!(dto.user_name, "Name");
    assert!(dto.granted_at.contains("T"));
}

#[test]
fn whitelist_entry_to_dto_preserves_fields() {
    let w = VoiceChannelWhitelistEntry {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        owner_id: "o".into(),
        target_id: "t".into(),
        target_name: "T".into(),
        created_at: Utc::now(),
    };
    let dto: WhitelistEntryResponseDto = w.into();
    assert_eq!(dto.target_id, "t");
    assert_eq!(dto.target_name, "T");
}

#[test]
fn ban_to_dto_preserves_fields() {
    let b = VoiceChannelBan {
        id: Uuid::new_v4(),
        voice_channel_id: Uuid::new_v4(),
        guild_id: "g".into(),
        owner_id: "owner".into(),
        user_id: "bad".into(),
        user_name: "BadGuy".into(),
        banned_by: "mod".into(),
        reason: Some("toxic".into()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        created_at: Utc::now(),
    };
    let dto: BanResponseDto = b.into();
    assert_eq!(dto.user_id, "bad".into());
    assert_eq!(dto.reason.as_deref(), Some("toxic"));
    assert!(dto.expires_at.is_some());
}

#[test]
fn ban_permanent_to_dto_preserves_none_expires() {
    let b = VoiceChannelBan {
        id: Uuid::new_v4(),
        voice_channel_id: Uuid::new_v4(),
        guild_id: "g".into(),
        owner_id: "owner".into(),
        user_id: "u".into(),
        user_name: "U".into(),
        banned_by: "m".into(),
        reason: None,
        expires_at: None,
        created_at: Utc::now(),
    };
    let dto: BanResponseDto = b.into();
    assert!(dto.reason.is_none());
    assert!(dto.expires_at.is_none());
}

#[test]
fn create_theme_dto_defaults_when_fields_omitted() {
    let dto: CreateThemeDto = serde_json::from_value(serde_json::json!({
        "name": "X"
    }))
    .unwrap();
    assert_eq!(dto.channel_name_template, "{user}");
    assert_eq!(dto.visibility, "visible");
    assert!(!dto.locked);
    assert!(!dto.queue_enabled);
    assert!(!dto.stage_enabled);
    assert!(!dto.is_default);
    assert_eq!(dto.sort_order, 0);
}
