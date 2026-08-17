//! Client HTTP du bot vers `nexus-api`.
//!
//! Le bot utilise ce client pour le portefeuille, la roue, le Coussin Piégé,
//! les jeux mentionnables et le portail de serveurs. Il ne lit jamais la base
//! directement et transforme les erreurs HTTP en messages exploitables par
//! les commandes Discord.

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

pub struct ApiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WheelSpinRequest {
    pub username: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WheelSpinResponse {
    pub case_label: String,
    pub payout: i64,
    pub balance_after: i64,
    pub is_memorable: bool,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WalletResponse {
    pub user_id: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
}

#[derive(Debug, Serialize)]
pub struct TransferRequest {
    pub from_user_id: String,
    pub from_username: String,
    pub to_user_id: String,
    pub to_username: String,
    pub amount: i64,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GrandSalonJoinRequest {
    pub display_name: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct GrandSalonProfileResponse {
    pub display_name: String,
    pub rayonnement: i64,
    pub jetons: i64,
    pub reputation: i64,
    pub bons_plans: i64,
    pub reseau: i64,
}
#[derive(Debug, Serialize)]
pub struct CoussinChallengeRequest {
    pub channel_id: String,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
}
#[derive(Debug, Deserialize)]
pub struct CoussinChallengeResponse {
    pub id: String,
    pub mise: i64,
}

#[derive(Debug, Serialize)]
pub struct CoussinDefenderRequest {
    pub defender_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoussinProfileResponse {
    pub username: String,
    pub class: String,
    pub level: i32,
    pub xp: i64,
    pub atk: i32,
    pub def: i32,
    pub hp_current: i32,
    pub hp_max: i32,
    pub coins: i64,
    pub stat_points: i32,
    pub title: String,
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub chaos_events: i32,
}

#[derive(Debug, Serialize)]
pub struct CoussinClassRequest {
    pub username: String,
    pub class: String,
}
#[derive(Debug, Serialize)]
pub struct CoussinTrainRequest {
    pub username: String,
    pub stat: String,
}
#[derive(Debug, Serialize)]
pub struct CoussinBuyItemRequest {
    pub item_key: String,
}
#[derive(Debug, Serialize)]
pub struct CoussinStealRequest {
    pub thief_name: String,
    pub victim_id: String,
    pub victim_name: String,
    /// Salon de la fouille : le denouement doit pouvoir y etre publie meme si
    /// le bot redemarre pendant la fenetre de defense.
    pub channel_id: String,
}

/// Fouille ouverte. Seul ce dont le bot a besoin pour poser son bouton : la
/// reponse de l'API en dit plus, mais deserialiser ce qu'on n'affiche pas ne
/// ferait que du code mort.
#[derive(Debug, serde::Deserialize)]
pub struct CoussinStealOpened {
    pub attempt_id: String,
    pub defense_window_seconds: i64,
}

/// Denouement d'une fouille resolue par un clic, avec le detail du jet.
///
/// La resolution DIFFEREE (fenetre fermee sans reaction) ne passe pas par ici :
/// elle arrive par le bus et se lit en JSON brut dans `coussin_steal_events`.
#[derive(Debug, serde::Deserialize)]
pub struct CoussinStealOutcome {
    pub thief_id: String,
    pub success: bool,
    pub amount: i64,
    pub thief_total: i32,
    pub victim_total: i32,
}
#[derive(Debug, Serialize)]
pub struct CoussinPrimeRequest {
    pub target_id: String,
    pub target_name: String,
    pub placer_name: String,
    pub amount: i64,
}
#[derive(Debug, Deserialize)]
pub struct CoussinInventoryItem {
    pub item_key: String,
    pub quantity: i32,
}
#[derive(Debug, Serialize)]
pub struct CoussinBetRequest {
    pub combat_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransferResponse {
    pub amount: i64,
    pub from_balance: i64,
}

/// URL-encode un segment de path pour eviter qu'un nom de jeu avec `/` ou
/// caracteres speciaux ne casse le routing ou ne permette une injection.
fn encode_segment(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

// ── Types du module games (catalogue + panels) ──

#[derive(Debug, Deserialize, Clone)]
pub struct Game {
    pub id: String,
    pub game_name: String,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub role_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GamePanel {
    pub id: String,
    pub channel_id: String,
    pub message_id: String,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Serialize)]
struct SavePanelReq<'a> {
    channel_id: &'a str,
    message_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<&'a str>,
}

// ── Types du module game-portal (serveurs de jeu) ──

#[derive(Debug, Deserialize)]
pub struct ServerDetailResponse {
    pub server: GameServer,
    #[serde(default)]
    pub config: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct GameServer {
    pub id: String,
    pub guild_id: String,
    pub template_id: String,
    pub name: String,
    pub status: String,
    pub owner_user_id: String,
    pub host_port: Option<u16>,
    pub public_host: Option<String>,
    pub ip_reveal_at: Option<String>,
    pub ip_revealed: bool,
    pub text_channel_id: Option<String>,
    pub voice_channel_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GameTemplate {
    pub slug: String,
    pub name: String,
    pub cover_image_url: Option<String>,
}

/// Reglage d'un template pour une guild : role Discord a pinguer.
#[derive(Debug, Deserialize)]
pub struct TemplateSettings {
    pub template_slug: String,
    pub discord_role_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ServerRegistration {
    pub user_id: String,
}

/// Réponse de `/reveal-ip/request` : décompte à annoncer + démarrage effectué.
#[derive(Debug, Deserialize)]
pub struct RevealRequest {
    pub delay_minutes: i64,
    pub started: bool,
}

mod achievements;
mod coussin;
mod economy;
mod game_portal;
mod games;

pub use achievements::AchievementProgress;

impl ApiClient {
    /// `base_url` ex. http://nexus-api:3100 (NEXUS_API_URL).
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    /// Envoie la requete, mappe 4xx/5xx vers un message affichable.
    async fn send<T: serde::de::DeserializeOwned>(
        &self,
        mut req: reqwest::RequestBuilder,
    ) -> Result<T, String> {
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("nexus-api injoignable: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            resp.json::<T>()
                .await
                .map_err(|e| format!("reponse nexus-api invalide: {e}"))
        } else {
            Err(resp
                .json::<ApiErrorBody>()
                .await
                .map(|b| b.error)
                .unwrap_or_else(|_| format!("erreur nexus-api ({status})")))
        }
    }

    /// Comme `send`, mais sans corps de reponse attendu (2xx => Ok).
    async fn send_no_content(&self, mut req: reqwest::RequestBuilder) -> Result<(), String> {
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("nexus-api injoignable: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(resp
                .json::<ApiErrorBody>()
                .await
                .map(|b| b.error)
                .unwrap_or_else(|_| format!("erreur nexus-api ({status})")))
        }
    }

    // ── Games : catalogue ──
}
