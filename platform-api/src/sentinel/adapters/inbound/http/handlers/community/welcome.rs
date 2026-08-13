use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::bootstrap::state::CommunityState;
use axum::extract::State;
use axum::Json;
use platform_core::sentinel::domain::entities::community::age_check::AgeCheckDecision;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::ports::inbound::community::manage_welcome_config::WelcomeConfigPatch;
use platform_core::sentinel::ports::outbound::community::welcome_config_repository::WelcomeConfigData;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct WelcomeConfigDto {
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
    pub age_check_enabled: bool,
    pub age_minimum: i32,
    pub unverified_role_id: Option<String>,
    pub age_modal_question: String,
    pub age_ban_message: String,
    pub age_min: i32,
    pub age_max: i32,
    pub age_ban_days_per_year: i32,
    pub age_ban_log_channel_id: Option<String>,
    pub leave_embed_color: String,
    pub rules_embed_color: String,
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
    pub welcome_title: String,
    pub welcome_image_url: String,
    pub welcome_footer_text: String,
    pub rejoin_title: String,
    pub rejoin_image_url: String,
    pub rejoin_footer_text: String,
    pub leave_title: String,
    pub leave_image_url: String,
    pub leave_footer_text: String,
    pub anniversary_title: String,
    pub anniversary_image_url: String,
    pub anniversary_footer_text: String,
}

impl From<WelcomeConfigData> for WelcomeConfigDto {
    fn from(c: WelcomeConfigData) -> Self {
        Self {
            guild_id: c.guild_id,
            welcome_enabled: c.welcome_enabled,
            welcome_channel_id: c.welcome_channel_id,
            welcome_message: c.welcome_message,
            welcome_embed_color: c.welcome_embed_color,
            welcome_dm_enabled: c.welcome_dm_enabled,
            welcome_dm_message: c.welcome_dm_message,
            leave_enabled: c.leave_enabled,
            leave_channel_id: c.leave_channel_id,
            leave_message: c.leave_message,
            rules_enabled: c.rules_enabled,
            rules_channel_id: c.rules_channel_id,
            rules_message: c.rules_message,
            rules_role_id: c.rules_role_id,
            rules_button_label: c.rules_button_label,
            age_check_enabled: c.age_check_enabled,
            age_minimum: c.age_minimum,
            unverified_role_id: c.unverified_role_id,
            age_modal_question: c.age_modal_question,
            age_ban_message: c.age_ban_message,
            age_min: c.age_min,
            age_max: c.age_max,
            age_ban_days_per_year: c.age_ban_days_per_year,
            age_ban_log_channel_id: c.age_ban_log_channel_id,
            leave_embed_color: c.leave_embed_color,
            rules_embed_color: c.rules_embed_color,
            counter_enabled: c.counter_enabled,
            counter_channel_id: c.counter_channel_id,
            counter_format: c.counter_format,
            voice_counter_enabled: c.voice_counter_enabled,
            voice_counter_channel_id: c.voice_counter_channel_id,
            voice_counter_format: c.voice_counter_format,
            anniversary_enabled: c.anniversary_enabled,
            anniversary_channel_id: c.anniversary_channel_id,
            anniversary_message: c.anniversary_message,
            rejoin_message: c.rejoin_message,
            welcome_title: c.welcome_title,
            welcome_image_url: c.welcome_image_url,
            welcome_footer_text: c.welcome_footer_text,
            rejoin_title: c.rejoin_title,
            rejoin_image_url: c.rejoin_image_url,
            rejoin_footer_text: c.rejoin_footer_text,
            leave_title: c.leave_title,
            leave_image_url: c.leave_image_url,
            leave_footer_text: c.leave_footer_text,
            anniversary_title: c.anniversary_title,
            anniversary_image_url: c.anniversary_image_url,
            anniversary_footer_text: c.anniversary_footer_text,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SaveWelcomeConfigDto {
    pub welcome_enabled: Option<bool>,
    pub welcome_channel_id: Option<String>,
    pub welcome_message: Option<String>,
    pub welcome_embed_color: Option<String>,
    pub welcome_dm_enabled: Option<bool>,
    pub welcome_dm_message: Option<String>,
    pub leave_enabled: Option<bool>,
    pub leave_channel_id: Option<String>,
    pub leave_message: Option<String>,
    pub rules_enabled: Option<bool>,
    pub rules_channel_id: Option<String>,
    pub rules_message: Option<String>,
    pub rules_role_id: Option<String>,
    pub rules_button_label: Option<String>,
    pub age_check_enabled: Option<bool>,
    pub age_minimum: Option<i32>,
    pub unverified_role_id: Option<String>,
    pub age_modal_question: Option<String>,
    pub age_ban_message: Option<String>,
    pub age_min: Option<i32>,
    pub age_max: Option<i32>,
    pub age_ban_days_per_year: Option<i32>,
    pub age_ban_log_channel_id: Option<String>,
    pub leave_embed_color: Option<String>,
    pub rules_embed_color: Option<String>,
    pub counter_enabled: Option<bool>,
    pub counter_channel_id: Option<String>,
    pub counter_format: Option<String>,
    pub voice_counter_enabled: Option<bool>,
    pub voice_counter_channel_id: Option<String>,
    pub voice_counter_format: Option<String>,
    pub anniversary_enabled: Option<bool>,
    pub anniversary_channel_id: Option<String>,
    pub anniversary_message: Option<String>,
    pub rejoin_message: Option<String>,
    // Titres / images / pieds d'embed : etaient absents du DTO -> ignores par
    // serde et forces a None dans dto_to_patch, donc jamais persistes (l'URL
    // d'image "disparaissait" apres sauvegarde).
    pub welcome_title: Option<String>,
    pub welcome_image_url: Option<String>,
    pub welcome_footer_text: Option<String>,
    pub leave_title: Option<String>,
    pub leave_image_url: Option<String>,
    pub leave_footer_text: Option<String>,
    pub anniversary_title: Option<String>,
    pub anniversary_image_url: Option<String>,
    pub anniversary_footer_text: Option<String>,
    pub rejoin_title: Option<String>,
    pub rejoin_image_url: Option<String>,
    pub rejoin_footer_text: Option<String>,
}

/// GET /api/welcome/{guild_id}
pub async fn get_config(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<WelcomeConfigDto>, ApiError> {
    // La config expose salons de logs, roles auto, templates -> lecture reservee
    // aux moderateurs du serveur (avant : aucun RBAC -> lecture cross-serveur).
    let config = state.welcome_config_uc.get(&guild_id).await?;
    Ok(Json(config.into()))
}

/// PUT /api/welcome/{guild_id}
pub async fn save_config(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<SaveWelcomeConfigDto>,
) -> Result<Json<WelcomeConfigDto>, ApiError> {
    let saved = state
        .welcome_config_uc
        .save_patch(&guild_id, dto_to_patch(dto))
        .await?;
    Ok(Json(saved.into()))
}

/// POST /api/welcome/{guild_id}/rules/publish
/// Demande au bot de (re)poster le panneau de reglement (texte + bouton
/// d'acceptation) dans le salon configure, via la stream d'events Redis.
pub async fn publish_rules(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Garde-fou : refuse si la validation du reglement n'est pas activee /
    // configuree (sinon le bot echouerait silencieusement cote consumer).
    let config = state.welcome_config_uc.get(&guild_id).await?;
    if !config.rules_enabled {
        return Err(ApiError::from(
            platform_core::sentinel::domain::errors::DomainError::ValidationError(
                "Active d'abord la validation du reglement.".into(),
            ),
        ));
    }
    if config.rules_channel_id.as_deref().unwrap_or("").is_empty() {
        return Err(ApiError::from(
            platform_core::sentinel::domain::errors::DomainError::ValidationError(
                "Choisis d'abord le salon du reglement.".into(),
            ),
        ));
    }
    state.broadcaster.broadcast(
        "welcome_rules_publish",
        serde_json::json!({ "guild_id": guild_id }),
    );
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Corps de `POST /api/welcome/{guild_id}/age-check` : le bot transmet l'age
/// declare (deja borne cote saisie) et l'utilisateur concerne.
#[derive(Debug, Deserialize)]
pub struct AgeCheckRequestDto {
    pub user_id: String,
    pub declared_age: i32,
}

/// Decision server-side renvoyee au bot. Le bot n'execute que l'action Discord
/// correspondante (grant role / ban + deban programme) et le rendu des messages.
#[derive(Debug, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum AgeCheckDecisionDto {
    /// Age suffisant -> le bot assigne le role membre.
    Grant,
    /// Age insuffisant -> le bot bannit jusqu'a `unban_at`.
    Ban {
        years: i32,
        /// Date de deban (RFC3339).
        unban_at: String,
        reason: String,
    },
}

impl From<AgeCheckDecision> for AgeCheckDecisionDto {
    fn from(d: AgeCheckDecision) -> Self {
        match d {
            AgeCheckDecision::Grant => AgeCheckDecisionDto::Grant,
            AgeCheckDecision::Ban {
                years,
                unban_at,
                reason,
            } => AgeCheckDecisionDto::Ban {
                years,
                unban_at: unban_at.to_rfc3339(),
                reason,
            },
        }
    }
}

/// POST /api/welcome/{guild_id}/age-check
/// Decide l'issue de la verification d'age (seuil pass/ban + duree de ban)
/// server-side. Le bot applique ensuite l'action Discord.
pub async fn age_check(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<AgeCheckRequestDto>,
) -> Result<Json<AgeCheckDecisionDto>, ApiError> {
    // Le bot passe en Internal -> bypass RBAC ; un appelant web doit etre
    // moderateur+ du serveur (parite avec l'enregistrement age-ban).
    let decision = state
        .age_check_uc
        .evaluate(&guild_id, &dto.user_id, dto.declared_age)
        .await?;
    Ok(Json(decision.into()))
}

fn dto_to_patch(dto: SaveWelcomeConfigDto) -> WelcomeConfigPatch {
    WelcomeConfigPatch {
        welcome_enabled: dto.welcome_enabled,
        welcome_channel_id: dto.welcome_channel_id,
        welcome_message: dto.welcome_message,
        welcome_embed_color: dto.welcome_embed_color,
        welcome_dm_enabled: dto.welcome_dm_enabled,
        welcome_dm_message: dto.welcome_dm_message,
        welcome_title: dto.welcome_title,
        welcome_image_url: dto.welcome_image_url,
        welcome_footer_text: dto.welcome_footer_text,
        leave_enabled: dto.leave_enabled,
        leave_channel_id: dto.leave_channel_id,
        leave_message: dto.leave_message,
        leave_title: dto.leave_title,
        leave_image_url: dto.leave_image_url,
        leave_footer_text: dto.leave_footer_text,
        rules_enabled: dto.rules_enabled,
        rules_channel_id: dto.rules_channel_id,
        rules_message: dto.rules_message,
        rules_role_id: dto.rules_role_id,
        rules_button_label: dto.rules_button_label,
        age_check_enabled: dto.age_check_enabled,
        age_minimum: dto.age_minimum,
        unverified_role_id: dto.unverified_role_id,
        age_modal_question: dto.age_modal_question,
        age_ban_message: dto.age_ban_message,
        age_min: dto.age_min,
        age_max: dto.age_max,
        age_ban_days_per_year: dto.age_ban_days_per_year,
        age_ban_log_channel_id: dto.age_ban_log_channel_id,
        leave_embed_color: dto.leave_embed_color,
        rules_embed_color: dto.rules_embed_color,
        counter_enabled: dto.counter_enabled,
        counter_channel_id: dto.counter_channel_id,
        counter_format: dto.counter_format,
        voice_counter_enabled: dto.voice_counter_enabled,
        voice_counter_channel_id: dto.voice_counter_channel_id,
        voice_counter_format: dto.voice_counter_format,
        anniversary_enabled: dto.anniversary_enabled,
        anniversary_channel_id: dto.anniversary_channel_id,
        anniversary_message: dto.anniversary_message,
        anniversary_title: dto.anniversary_title,
        anniversary_image_url: dto.anniversary_image_url,
        anniversary_footer_text: dto.anniversary_footer_text,
        rejoin_message: dto.rejoin_message,
        rejoin_title: dto.rejoin_title,
        rejoin_image_url: dto.rejoin_image_url,
        rejoin_footer_text: dto.rejoin_footer_text,
    }
}

#[cfg(test)]
#[path = "tests/welcome.rs"]
mod tests;
