use platform_core::sentinel::domain::entities::ai::image_analysis::ImageAnalysis;
use platform_core::sentinel::domain::entities::system::discord_ids::ChannelId;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::MessageId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct AnalyzeImageRequestDto {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub username: String,
    pub message_id: MessageId,
    /// Image encodee en base64
    pub image_data: String,
    pub content_type: String,
    pub filename: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeImageResponseDto {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    pub classifications: Vec<ClassificationDto>,
}

#[derive(Debug, Serialize)]
pub struct ClassificationDto {
    pub label: String,
    pub confidence: f32,
}

impl From<ImageAnalysis> for AnalyzeImageResponseDto {
    fn from(analysis: ImageAnalysis) -> Self {
        Self {
            action: analysis.action.as_str().to_string(),
            reason: if analysis.reason.is_empty() {
                None
            } else {
                Some(analysis.reason)
            },
            duration: analysis.duration,
            classifications: analysis
                .classifications
                .into_iter()
                .map(|c| ClassificationDto {
                    label: c.label,
                    confidence: c.confidence,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
#[path = "tests/analyze_image.rs"]
mod tests;
