//! Wrappers Postgres pour les enums du domaine.
//!
//! `platform_core::sentinel` ne connaît pas SQLx : les dérivations `sqlx::Type` qui
//! lient les enums aux types Postgres custom (`moderation_gravity`,
//! `voice_channel_kind`) vivent ici, dans l'adapter. Les
//! repos `query_as!` decodent vers `Pg*` puis convertissent via `.into()`.

use platform_core::sentinel::domain::enums::community::voice_channel_kind::VoiceChannelKind;
use platform_core::sentinel::domain::enums::moderation::moderation_gravity::ModerationGravity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "moderation_gravity", rename_all = "lowercase")]
pub enum PgModerationGravity {
    Low,
    Medium,
    High,
    Critical,
}

impl From<PgModerationGravity> for ModerationGravity {
    fn from(g: PgModerationGravity) -> Self {
        match g {
            PgModerationGravity::Low => Self::Low,
            PgModerationGravity::Medium => Self::Medium,
            PgModerationGravity::High => Self::High,
            PgModerationGravity::Critical => Self::Critical,
        }
    }
}

impl From<ModerationGravity> for PgModerationGravity {
    fn from(g: ModerationGravity) -> Self {
        match g {
            ModerationGravity::Low => Self::Low,
            ModerationGravity::Medium => Self::Medium,
            ModerationGravity::High => Self::High,
            ModerationGravity::Critical => Self::Critical,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, sqlx::Type)]
#[sqlx(type_name = "voice_channel_kind", rename_all = "lowercase")]
pub enum PgVoiceChannelKind {
    #[default]
    Public,
    Private,
}

impl From<PgVoiceChannelKind> for VoiceChannelKind {
    fn from(k: PgVoiceChannelKind) -> Self {
        match k {
            PgVoiceChannelKind::Public => Self::Public,
            PgVoiceChannelKind::Private => Self::Private,
        }
    }
}

impl From<VoiceChannelKind> for PgVoiceChannelKind {
    fn from(k: VoiceChannelKind) -> Self {
        match k {
            VoiceChannelKind::Public => Self::Public,
            VoiceChannelKind::Private => Self::Private,
        }
    }
}
