use sqlx::PgPool;

pub struct PgVoiceChannelRepository {
    pool: PgPool,
}

impl PgVoiceChannelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

mod bans;
mod channels;
mod co_admins;
mod invites;
mod presets;
mod themes;
mod whitelist;
