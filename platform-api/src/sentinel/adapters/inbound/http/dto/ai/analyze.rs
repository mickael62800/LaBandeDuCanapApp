use platform_core::sentinel::domain::entities::ai::message_analysis::MessageAnalysis;
use platform_core::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use platform_core::sentinel::domain::entities::system::discord_ids::ChannelId;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::MessageId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;
use platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageCommand;
use serde::Deserialize;
use serde::Serialize;

/// DTO de la requête reçue depuis le bot automod.
#[derive(Debug, Deserialize)]
pub struct AnalyzeRequestDto {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub username: String,
    pub content: String,
    pub flags: DetectionFlags,
    pub metadata: MetadataDto,
    /// Messages de contexte conversationnel pour l'analyse de sentiment.
    pub context_messages: Vec<ContextMessageDto>,
}

#[derive(Debug, Deserialize)]
pub struct MetadataDto {
    pub message_id: MessageId,
    pub timestamp: String,
}

/// Message de contexte envoye par le bot pour l'analyse de sentiment.
#[derive(Debug, Deserialize)]
pub struct ContextMessageDto {
    pub username: String,
    pub content: String,
}

/// DTO de la réponse renvoyée au bot.
#[derive(Debug, Serialize)]
pub struct AnalyzeResponseDto {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

impl From<AnalyzeRequestDto> for AnalyzeMessageCommand {
    fn from(dto: AnalyzeRequestDto) -> Self {
        // Tronquer le contenu a 2500 chars (Discord = 2000 + marge).
        // H9 — logger explicitement si truncation : si un lien phishing est en
        // fin de message long, on sait qu'il a ete coupe.
        let content = if dto.content.len() > 2500 {
            tracing::warn!(
                guild_id = %dto.guild_id,
                user_id = %dto.user_id,
                original_len = dto.content.len(),
                "Contenu analyse tronque a 2500 chars (perte potentielle d'indices en queue)"
            );
            dto.content.chars().take(2500).collect()
        } else {
            dto.content
        };
        let context_messages = dto
            .context_messages
            .into_iter()
            .map(|m| {
                platform_core::sentinel::ports::inbound::ai::analyze_message::ContextMessageEntry {
                    username: m.username,
                    content: m.content,
                }
            })
            .collect();

        Self {
            guild_id: dto.guild_id,
            channel_id: dto.channel_id,
            user_id: dto.user_id,
            username: dto.username,
            content,
            flags: dto.flags,
            message_id: dto.metadata.message_id,
            timestamp: dto.metadata.timestamp,
            context_messages,
        }
    }
}

impl From<MessageAnalysis> for AnalyzeResponseDto {
    fn from(analysis: MessageAnalysis) -> Self {
        Self {
            action: analysis.action.as_str().to_string(),
            reason: if analysis.reason.is_empty() {
                None
            } else {
                Some(analysis.reason)
            },
            duration: analysis.duration,
        }
    }
}

#[cfg(test)]
#[path = "tests/analyze.rs"]
mod tests;
