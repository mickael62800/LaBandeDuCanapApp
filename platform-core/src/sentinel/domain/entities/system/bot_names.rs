//! Noms canoniques des modules (`bot_name` dans `bot_guild_config` /
//! `bot_definitions`). Ces chaînes sont des clés de jointure : une typo est
//! une config silencieusement vide, pas une erreur de compilation — d'où des
//! constantes plutôt que des littéraux nus.

pub const ANALYTICS_BOT: &str = "analytics";
pub const ANNOUNCEMENTS_BOT: &str = "announcements";
pub const AUDIT_BOT: &str = "audit-bot";
pub const AUTOMOD_BOT: &str = "automod-bot";
pub const BUMP_BOT: &str = "bump-bot";
pub const COMMUNITY_BOT: &str = "community-bot";
pub const GUILD_BACKUP_BOT: &str = "guild-backup-bot";
pub const MODERATION_BOT: &str = "moderation-bot";
pub const PROGRESSION_BOT: &str = "progression-bot";
pub const SECURITY_BOT: &str = "security-bot";
pub const TICKET_BOT: &str = "ticket-bot";
pub const WELCOME_BOT: &str = "welcome-bot";
