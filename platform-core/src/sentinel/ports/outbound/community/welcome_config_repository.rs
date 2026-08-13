use async_trait::async_trait;

use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::errors::DomainError;

/// Config welcome brute (1 row par guild). Les defaults sont appliques
/// par le repository si la row n'existe pas.
#[derive(Debug, Clone)]
pub struct WelcomeConfigData {
    pub guild_id: GuildId,
    pub welcome_enabled: bool,
    pub welcome_channel_id: Option<String>,
    pub welcome_message: String,
    pub welcome_embed_color: String,
    pub welcome_dm_enabled: bool,
    pub welcome_dm_message: String,
    pub leave_enabled: bool,
    pub leave_channel_id: Option<String>,
    pub leave_message: String,
    pub rules_enabled: bool,
    pub rules_channel_id: Option<String>,
    pub rules_message: String,
    pub rules_role_id: Option<String>,
    pub rules_button_label: String,
    pub rules_embed_color: String,
    // Verification d'age au reglement.
    pub age_check_enabled: bool,
    pub age_minimum: i32,
    pub unverified_role_id: Option<String>,
    pub age_modal_question: String,
    pub age_ban_message: String,
    // Verification d'age — bornes de saisie + parametrage du ban. Ces cles sont
    // lues DIRECTEMENT par le bot via get_guild_config_for("welcome-bot") (elles
    // ne transitent PAS par le proto gRPC) ; le repo doit seulement les
    // (re)ecrire dans bot_guild_config pour que la lecture directe du bot les voie.
    pub age_min: i32,
    pub age_max: i32,
    pub age_ban_days_per_year: i32,
    pub age_ban_log_channel_id: Option<String>,
    pub counter_enabled: bool,
    pub counter_channel_id: Option<String>,
    pub counter_format: String,
    pub voice_counter_enabled: bool,
    pub voice_counter_channel_id: Option<String>,
    pub voice_counter_format: String,
    pub anniversary_enabled: bool,
    pub anniversary_channel_id: Option<String>,
    pub anniversary_message: String,
    pub rejoin_message: String,
    // Embed enrichi — bienvenue
    pub welcome_title: String,
    pub welcome_image_url: String,
    pub welcome_footer_text: String,
    /// Position du texte / a l'image ("above" | "below", defaut "below").
    /// L'image et le texte partent en deux messages distincts.
    // Embed enrichi — retour (rejoin)
    pub rejoin_title: String,
    pub rejoin_image_url: String,
    pub rejoin_footer_text: String,
    // Embed enrichi — depart
    pub leave_title: String,
    pub leave_image_url: String,
    pub leave_footer_text: String,
    pub leave_embed_color: String,
    // Embed enrichi — anniversaire
    pub anniversary_title: String,
    pub anniversary_image_url: String,
    pub anniversary_footer_text: String,
}

#[async_trait]
pub trait WelcomeConfigRepository: Send + Sync {
    async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfigData, DomainError>;
    async fn save_config(
        &self,
        guild_id: &str,
        data: &WelcomeConfigData,
    ) -> Result<WelcomeConfigData, DomainError>;
}
