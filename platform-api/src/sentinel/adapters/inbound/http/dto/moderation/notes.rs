use platform_core::sentinel::domain::entities::moderation::user_note::UserNote;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;
use platform_core::sentinel::ports::inbound::moderation::manage_notes::AddNoteCommand;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct AddNoteDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    #[serde(default = "default_category")]
    pub category: String,
}

fn default_category() -> String {
    "general".into()
}

impl From<AddNoteDto> for AddNoteCommand {
    fn from(dto: AddNoteDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            user_id: dto.user_id,
            author_id: dto.author_id,
            author_name: dto.author_name,
            content: dto.content,
            category: dto.category,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserNoteDto {
    pub id: String,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub category: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<UserNote> for UserNoteDto {
    fn from(n: UserNote) -> Self {
        Self {
            id: n.id.to_string(),
            guild_id: n.guild_id,
            user_id: n.user_id,
            author_id: n.author_id,
            author_name: n.author_name,
            content: n.content,
            category: n.category,
            created_at: n.created_at.to_rfc3339(),
            updated_at: n.updated_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
#[path = "tests/notes.rs"]
mod tests;
