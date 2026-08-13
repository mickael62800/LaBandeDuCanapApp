use super::*;

use chrono::TimeZone;
use platform_core::sentinel::domain::enums::community::voice_channel_kind::VoiceChannelKind;
use uuid::Uuid;

fn ts() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

fn sample_channel(kind: VoiceChannelKind) -> VoiceChannel {
    VoiceChannel {
        id: Uuid::nil(),
        guild_id: "g".into(),
        owner_id: "u".into(),
        owner_name: "Joe".into(),
        channel_id: "ch".into(),
        text_channel_id: Some("t".into()),
        members_channel_id: Some("m".into()),
        queue_channel_id: None,
        category_id: Some("cat".into()),
        channel_name: "Salon Joe".into(),
        kind,
        visibility: "public".into(),
        queue_enabled: false,
        locked: false,
        stage_enabled: false,
        member_limit: Some(10),
        status: Some("active".into()),
        channel_status: "active".into(),
        closed_at: None,
        created_at: ts(),
    }
}

#[test]
fn voice_channel_to_proto_public() {
    let p = voice_channel_to_proto(sample_channel(VoiceChannelKind::Public));
    assert_eq!(p.kind, "public");
    assert_eq!(p.member_limit, Some(10));
    assert_eq!(p.text_channel_id.as_deref(), Some("t"));
    assert!(p.queue_channel_id.is_none());
    assert_eq!(p.created_at, ts().to_rfc3339());
}

#[test]
fn voice_channel_to_proto_private() {
    let p = voice_channel_to_proto(sample_channel(VoiceChannelKind::Private));
    assert_eq!(p.kind, "private");
}

#[test]
fn voice_theme_to_proto_full_mapping() {
    use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelTheme;
    let id = Uuid::new_v4();
    let theme = VoiceChannelTheme {
        id,
        guild_id: "g1".into(),
        name: "Gaming".into(),
        emoji: Some("🎮".into()),
        channel_name_template: "{user}'s Game".into(),
        member_limit: Some(5),
        visibility: "visible".into(),
        locked: false,
        queue_enabled: true,
        bitrate: Some(96000),
        slowmode_secs: Some(10),
        stage_enabled: false,
        is_default: true,
        sort_order: 3,
        created_at: ts(),
    };
    let p = voice_theme_to_proto(theme);
    assert_eq!(p.id, id.to_string());
    assert_eq!(p.guild_id, "g1");
    assert_eq!(p.name, "Gaming");
    assert_eq!(p.emoji.as_deref(), Some("🎮"));
    assert_eq!(p.channel_name_template, "{user}'s Game");
    assert_eq!(p.member_limit, Some(5));
    assert_eq!(p.visibility, "visible");
    assert!(p.queue_enabled);
    assert_eq!(p.bitrate, Some(96000));
    assert_eq!(p.slowmode_secs, Some(10));
    assert!(p.is_default);
    assert_eq!(p.sort_order, 3);
    assert_eq!(p.created_at, ts().to_rfc3339());
}

#[test]
fn voice_theme_to_proto_minimal_optionals() {
    use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelTheme;
    let theme = VoiceChannelTheme {
        id: Uuid::nil(),
        guild_id: "g".into(),
        name: "Basic".into(),
        emoji: None,
        channel_name_template: "{user}".into(),
        member_limit: None,
        visibility: "hidden".into(),
        locked: true,
        queue_enabled: false,
        bitrate: None,
        slowmode_secs: None,
        stage_enabled: true,
        is_default: false,
        sort_order: 0,
        created_at: ts(),
    };
    let p = voice_theme_to_proto(theme);
    assert!(p.emoji.is_none());
    assert!(p.member_limit.is_none());
    assert!(p.bitrate.is_none());
    assert!(p.slowmode_secs.is_none());
    assert!(p.locked);
    assert!(p.stage_enabled);
}

#[test]
fn voice_channel_to_proto_locked_with_no_limit() {
    let mut c = sample_channel(VoiceChannelKind::Public);
    c.locked = true;
    c.member_limit = None;
    c.status = None;
    let p = voice_channel_to_proto(c);
    assert!(p.locked);
    assert!(p.member_limit.is_none());
    assert!(p.status.is_none());
}

// ── RPC tests avec mock ──

use async_trait::async_trait;
use chrono::Utc;
use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelBan;
use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelCoAdmin;
use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelConfig;
use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelDetail;
use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelInviteLink;
use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelTheme;
use platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelWhitelistEntry;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::BanFromChannelCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::CreateInviteLinkCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::CreateThemeCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::CreateVoiceChannelCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::ManageCoAdminCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::ManageVoiceChannelsUseCase;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::ManageWhitelistCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::TransferOwnershipCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::UpdateVoiceChannelCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::UseInviteLinkCommand;
use std::sync::Arc;
use std::sync::Mutex;
#[derive(Default)]
struct MockVoiceUc {
    channels: Mutex<Vec<VoiceChannel>>,
    detail: Mutex<Option<VoiceChannelDetail>>,
    create_calls: Mutex<Vec<CreateVoiceChannelCommand>>,
    delete_calls: Mutex<Vec<String>>,
    update_calls: Mutex<Vec<UpdateVoiceChannelCommand>>,
    transfer_calls: Mutex<Vec<TransferOwnershipCommand>>,
    add_co_admin_calls: Mutex<Vec<ManageCoAdminCommand>>,
    whitelist_calls: Mutex<Vec<ManageWhitelistCommand>>,
    ban_calls: Mutex<Vec<BanFromChannelCommand>>,
    themes: Mutex<Vec<VoiceChannelTheme>>,
    config: Mutex<Option<VoiceChannelConfig>>,
}

#[async_trait]
impl ManageVoiceChannelsUseCase for MockVoiceUc {
    async fn list_all_channels(&self) -> Result<Vec<VoiceChannel>, DomainError> {
        Ok(vec![])
    }
    async fn list_channels(&self, _: &str) -> Result<Vec<VoiceChannel>, DomainError> {
        Ok(self.channels.lock().unwrap().clone())
    }
    async fn list_history_channels(
        &self,
        _: &str,
        _: i64,
    ) -> Result<Vec<VoiceChannel>, DomainError> {
        Ok(vec![])
    }
    async fn get_channel_detail(&self, _: &str) -> Result<VoiceChannelDetail, DomainError> {
        self.detail
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| DomainError::NotFound("channel".into()))
    }
    async fn create_channel(
        &self,
        cmd: CreateVoiceChannelCommand,
    ) -> Result<VoiceChannel, DomainError> {
        let c = sample_channel(VoiceChannelKind::Public);
        self.create_calls.lock().unwrap().push(cmd);
        Ok(c)
    }
    async fn close_channel(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_channel(&self, id: &str) -> Result<(), DomainError> {
        self.delete_calls.lock().unwrap().push(id.into());
        Ok(())
    }
    async fn find_guild_id(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
    async fn purge_channel(&self, _: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn purge_history(&self, _: &str) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn update_channel(&self, cmd: UpdateVoiceChannelCommand) -> Result<(), DomainError> {
        self.update_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn transfer_ownership(&self, cmd: TransferOwnershipCommand) -> Result<(), DomainError> {
        self.transfer_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn add_co_admin(&self, cmd: ManageCoAdminCommand) -> Result<(), DomainError> {
        self.add_co_admin_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn remove_co_admin(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_whitelist(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError> {
        Ok(vec![])
    }
    async fn add_to_whitelist(&self, cmd: ManageWhitelistCommand) -> Result<(), DomainError> {
        self.whitelist_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn remove_from_whitelist(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_preset(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        Option<platform_core::sentinel::domain::entities::community::voice_channel::VoiceChannelPreset>,
        DomainError,
    > {
        Ok(None)
    }
    async fn save_preset(
        &self,
        _: platform_core::sentinel::ports::inbound::community::manage_voice_channels::SavePresetCommand,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn ban_from_channel(&self, cmd: BanFromChannelCommand) -> Result<(), DomainError> {
        self.ban_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn unban_from_channel(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn is_banned(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn list_owner_bans(&self, _: &str, _: &str) -> Result<Vec<VoiceChannelBan>, DomainError> {
        Ok(vec![])
    }
    async fn create_invite_link(
        &self,
        _: CreateInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError> {
        unimplemented!()
    }
    async fn list_invite_links(&self, _: &str) -> Result<Vec<VoiceChannelInviteLink>, DomainError> {
        Ok(vec![])
    }
    async fn use_invite_link(
        &self,
        _: UseInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError> {
        unimplemented!()
    }
    async fn revoke_invite_link(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_voice_config(&self, _g: &str) -> Result<VoiceChannelConfig, DomainError> {
        Ok(
            (*self.config.lock().unwrap()).unwrap_or(VoiceChannelConfig {
                creation_cooldown_secs: 60,
                flood_max_messages: 5,
                flood_time_window_secs: 10,
                empty_cleanup_delay_secs: 300,
                flood_mute_duration_secs: 300,
                vote_kick_timeout_secs: 120,
            }),
        )
    }
    async fn list_themes(&self, _: &str) -> Result<Vec<VoiceChannelTheme>, DomainError> {
        Ok(self.themes.lock().unwrap().clone())
    }
    async fn create_theme(&self, _: CreateThemeCommand) -> Result<VoiceChannelTheme, DomainError> {
        unimplemented!()
    }
    async fn update_theme(
        &self,
        _: &str,
        _: CreateThemeCommand,
    ) -> Result<VoiceChannelTheme, DomainError> {
        unimplemented!()
    }
    async fn delete_theme(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

fn grpc(uc: Arc<MockVoiceUc>) -> VoiceChannelsGrpc {
    VoiceChannelsGrpc { uc }
}

#[tokio::test]
async fn list_channels_returns_all() {
    let uc = Arc::new(MockVoiceUc::default());
    uc.channels
        .lock()
        .unwrap()
        .push(sample_channel(VoiceChannelKind::Public));
    let g = grpc(uc);
    let resp = g
        .list_channels(Request::new(proto::ListChannelsRequest {
            guild_id: "g".into(),
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().channels.len(), 1);
}

#[tokio::test]
async fn create_channel_delegates_command() {
    let uc = Arc::new(MockVoiceUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .create_channel(Request::new(proto::CreateChannelRequest {
            guild_id: "g".into(),
            owner_id: "o".into(),
            owner_name: "Owner".into(),
            channel_id: "c".into(),
            text_channel_id: None,
            members_channel_id: None,
            queue_channel_id: None,
            category_id: Some("cat".into()),
            channel_name: "New".into(),
            kind: "public".into(),
            visibility: "visible".into(),
            queue_enabled: false,
        }))
        .await
        .unwrap();
    let calls = uc.create_calls.lock().unwrap();
    assert_eq!(calls[0].owner_id, "o");
    assert_eq!(calls[0].kind, "public");
}

#[tokio::test]
async fn delete_channel_delegates_id() {
    let uc = Arc::new(MockVoiceUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .delete_channel(Request::new(proto::DeleteChannelRequest {
            channel_id: "ch1".into(),
        }))
        .await
        .unwrap();
    assert_eq!(uc.delete_calls.lock().unwrap()[0], "ch1");
}

#[tokio::test]
async fn update_channel_unwraps_optional_wrappers() {
    let uc = Arc::new(MockVoiceUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .update_channel(Request::new(proto::UpdateChannelRequest {
            channel_id: "c".into(),
            visibility: Some("visible".into()),
            locked: Some(true),
            queue_enabled: None,
            name: Some("new-name".into()),
            status: None,
            member_limit: Some(proto::MemberLimitUpdate { value: Some(10) }),
            queue_channel_id: None,
        }))
        .await
        .unwrap();
    let calls = uc.update_calls.lock().unwrap();
    assert_eq!(calls[0].locked, Some(true));
    assert_eq!(calls[0].member_limit, Some(Some(10)));
    assert_eq!(calls[0].name.as_deref(), Some("new-name"));
}

#[tokio::test]
async fn get_channel_not_found_returns_none() {
    let g = grpc(Arc::new(MockVoiceUc::default()));
    let resp = g
        .get_channel(Request::new(proto::GetChannelRequest {
            channel_id: "ghost".into(),
        }))
        .await
        .unwrap();
    let inner = resp.into_inner();
    assert!(inner.channel.is_none());
    assert!(inner.co_admins.is_empty());
}

#[tokio::test]
async fn get_channel_found_returns_detail_with_co_admins() {
    let uc = Arc::new(MockVoiceUc::default());
    let ch = sample_channel(VoiceChannelKind::Public);
    let chan_id = ch.id;
    *uc.detail.lock().unwrap() = Some(VoiceChannelDetail {
        channel: ch,
        co_admins: vec![VoiceChannelCoAdmin {
            id: Uuid::new_v4(),
            voice_channel_id: chan_id,
            user_id: "co1".into(),
            user_name: "CoAdmin".into(),
            granted_at: Utc::now(),
        }],
        bans: vec![],
        invite_links: vec![],
    });
    let _ = uc
        .detail
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .channel
        .channel_id
        .clone();
    // pour enlever le warning "unused"
    let _ = VoiceChannelBan {
        id: Uuid::new_v4(),
        voice_channel_id: Uuid::new_v4(),
        guild_id: String::new().into(),
        owner_id: String::new(),
        user_id: String::new().into(),
        user_name: String::new(),
        banned_by: String::new(),
        reason: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    let g = grpc(uc);
    let resp = g
        .get_channel(Request::new(proto::GetChannelRequest {
            channel_id: "ch".into(),
        }))
        .await
        .unwrap();
    let inner = resp.into_inner();
    assert!(inner.channel.is_some());
    assert_eq!(inner.co_admins.len(), 1);
    assert_eq!(inner.co_admins[0].user_name, "CoAdmin");
}

#[tokio::test]
async fn transfer_ownership_delegates() {
    let uc = Arc::new(MockVoiceUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .transfer_ownership(Request::new(proto::TransferOwnershipRequest {
            channel_id: "ch".into(),
            new_owner_id: "u2".into(),
            new_owner_name: "Owner2".into(),
        }))
        .await
        .unwrap();
    let calls = uc.transfer_calls.lock().unwrap();
    assert_eq!(calls[0].new_owner_id, "u2");
}

#[tokio::test]
async fn add_co_admin_delegates() {
    let uc = Arc::new(MockVoiceUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .add_co_admin(Request::new(proto::AddCoAdminRequest {
            channel_id: "ch".into(),
            user_id: "co".into(),
            user_name: "CoAdmin".into(),
        }))
        .await
        .unwrap();
    assert_eq!(
        uc.add_co_admin_calls.lock().unwrap()[0].user_name,
        "CoAdmin"
    );
}

#[tokio::test]
async fn add_to_whitelist_delegates() {
    let uc = Arc::new(MockVoiceUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .add_to_whitelist(Request::new(proto::AddToWhitelistRequest {
            guild_id: "g".into(),
            owner_id: "o".into(),
            target_id: "t".into(),
            target_name: "Target".into(),
        }))
        .await
        .unwrap();
    assert_eq!(uc.whitelist_calls.lock().unwrap()[0].target_name, "Target");
}

#[tokio::test]
async fn ban_from_channel_delegates_with_duration() {
    let uc = Arc::new(MockVoiceUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .ban_from_channel(Request::new(proto::BanFromChannelRequest {
            channel_id: "ch".into(),
            user_id: "u".into(),
            user_name: "BadUser".into(),
            banned_by: "owner".into(),
            reason: Some("spam".into()),
            duration_secs: Some(3600),
        }))
        .await
        .unwrap();
    let calls = uc.ban_calls.lock().unwrap();
    assert_eq!(calls[0].duration_secs, Some(3600));
    assert_eq!(calls[0].reason.as_deref(), Some("spam"));
}

#[tokio::test]
async fn get_voice_config_returns_proto() {
    let g = grpc(Arc::new(MockVoiceUc::default()));
    let resp = g
        .get_voice_config(Request::new(proto::GetVoiceConfigRequest {
            guild_id: "g".into(),
        }))
        .await
        .unwrap();
    let cfg = resp.into_inner();
    assert_eq!(cfg.creation_cooldown_secs, 60);
    assert_eq!(cfg.flood_max_messages, 5);
}

#[tokio::test]
async fn list_themes_returns_all() {
    let uc = Arc::new(MockVoiceUc::default());
    uc.themes.lock().unwrap().push(VoiceChannelTheme {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        name: "Gaming".into(),
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
        created_at: Utc::now(),
    });
    let g = grpc(uc);
    let resp = g
        .list_themes(Request::new(proto::ListThemesRequest {
            guild_id: "g".into(),
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().themes.len(), 1);
}
