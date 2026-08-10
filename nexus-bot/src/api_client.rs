//! Client HTTP minimal vers nexus-api (modele BaseApiClient simplifie).

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
}

#[derive(Debug, Deserialize)]
pub struct GameServer {
    pub guild_id: String,
    pub template_id: String,
    pub name: String,
    pub host_port: Option<u16>,
    pub ip_reveal_at: Option<String>,
    pub ip_revealed: bool,
    pub text_channel_id: Option<String>,
    pub voice_channel_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GameTemplate {
    pub slug: String,
    pub name: String,
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

    /// GET /api/games/{guild_id}.
    pub async fn list_games(&self, guild_id: &str) -> Result<Vec<Game>, String> {
        let url = format!("{}/api/games/{}", self.base_url, encode_segment(guild_id));
        self.send(self.http.get(&url)).await
    }

    /// GET /api/games/{guild_id}/by-category[?category=X] (None => sans categorie).
    pub async fn list_games_by_category(
        &self,
        guild_id: &str,
        category: Option<&str>,
    ) -> Result<Vec<Game>, String> {
        let mut url = format!(
            "{}/api/games/{}/by-category",
            self.base_url,
            encode_segment(guild_id)
        );
        if let Some(cat) = category {
            url.push_str(&format!("?category={}", encode_segment(cat)));
        }
        self.send(self.http.get(&url)).await
    }

    /// POST /api/games. Le bot cree d'abord le role Discord puis passe son ID.
    pub async fn create_game(
        &self,
        guild_id: &str,
        game_name: &str,
        created_by: &str,
        role_id: Option<&str>,
        emoji: Option<&str>,
        category: Option<&str>,
    ) -> Result<Game, String> {
        let url = format!("{}/api/games", self.base_url);
        let body = serde_json::json!({
            "guild_id": guild_id,
            "game_name": game_name,
            "created_by": created_by,
            "emoji": emoji,
            "category": category,
            "role_id": role_id,
        });
        self.send(self.http.post(&url).json(&body)).await
    }

    /// PUT /api/games/{guild_id}/{game_id}/role — associe un role Discord a un jeu
    /// existant (backfill des jeux legacy sans role).
    pub async fn set_game_role(
        &self,
        guild_id: &str,
        game_id: &str,
        role_id: &str,
    ) -> Result<Game, String> {
        let url = format!(
            "{}/api/games/{}/{}/role",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(game_id)
        );
        let body = serde_json::json!({ "role_id": role_id });
        self.send(self.http.put(&url).json(&body)).await
    }

    /// DELETE /api/games/{guild_id}/{game_id}.
    pub async fn delete_game(&self, guild_id: &str, game_id: &str) -> Result<(), String> {
        let url = format!(
            "{}/api/games/{}/{}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(game_id)
        );
        self.send_no_content(self.http.delete(&url)).await
    }

    /// GET /api/games/{guild_id}/by-name/{game_name}.
    pub async fn get_game_by_name(
        &self,
        guild_id: &str,
        game_name: &str,
    ) -> Result<Option<Game>, String> {
        let url = format!(
            "{}/api/games/{}/by-name/{}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(game_name)
        );
        self.send(self.http.get(&url)).await
    }

    // ── Games : panels ──

    /// POST /api/games/{guild_id}/panels.
    pub async fn save_panel(
        &self,
        guild_id: &str,
        channel_id: &str,
        message_id: &str,
        category: Option<&str>,
    ) -> Result<GamePanel, String> {
        let url = format!(
            "{}/api/games/{}/panels",
            self.base_url,
            encode_segment(guild_id)
        );
        let body = SavePanelReq {
            channel_id,
            message_id,
            category,
        };
        self.send(self.http.post(&url).json(&body)).await
    }

    /// GET /api/games/{guild_id}/panels.
    pub async fn list_panels(&self, guild_id: &str) -> Result<Vec<GamePanel>, String> {
        let url = format!(
            "{}/api/games/{}/panels",
            self.base_url,
            encode_segment(guild_id)
        );
        self.send(self.http.get(&url)).await
    }

    // ── Game portal : serveurs de jeu ──

    /// GET /api/games/servers/{server_id}.
    pub async fn get_game_server(&self, server_id: &str) -> Result<ServerDetailResponse, String> {
        let url = format!(
            "{}/api/games/servers/{}",
            self.base_url,
            encode_segment(server_id)
        );
        self.send(self.http.get(&url)).await
    }

    /// GET /api/games/templates/{template_id}.
    pub async fn get_game_template(&self, template_id: &str) -> Result<GameTemplate, String> {
        let url = format!(
            "{}/api/games/templates/{}",
            self.base_url,
            encode_segment(template_id)
        );
        self.send(self.http.get(&url)).await
    }

    /// POST /api/games/servers/{server_id}/registrations.
    pub async fn register_to_server(
        &self,
        server_id: &str,
        user_id: &str,
    ) -> Result<serde_json::Value, String> {
        let url = format!(
            "{}/api/games/servers/{}/registrations",
            self.base_url,
            encode_segment(server_id)
        );
        let body = serde_json::json!({ "user_id": user_id });
        self.send(self.http.post(&url).json(&body)).await
    }

    /// GET /api/games/servers/{server_id}/registrations.
    pub async fn list_server_registrations(
        &self,
        server_id: &str,
    ) -> Result<Vec<ServerRegistration>, String> {
        let url = format!(
            "{}/api/games/servers/{}/registrations",
            self.base_url,
            encode_segment(server_id)
        );
        self.send(self.http.get(&url)).await
    }

    /// GET /api/games/{guild_id}/template-settings — reglages par template
    /// (role Discord a pinguer).
    pub async fn list_template_settings(
        &self,
        guild_id: &str,
    ) -> Result<Vec<TemplateSettings>, String> {
        let url = format!(
            "{}/api/games/{}/template-settings",
            self.base_url,
            encode_segment(guild_id)
        );
        self.send(self.http.get(&url)).await
    }

    /// GET /api/config/{guild_id}/{bot_name} — config bot de la guild,
    /// aplatie en `cle -> valeur`.
    pub async fn get_guild_config(
        &self,
        guild_id: &str,
        bot_name: &str,
    ) -> Result<HashMap<String, String>, String> {
        let url = format!(
            "{}/api/config/{}/{}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(bot_name)
        );
        self.send(self.http.get(&url)).await
    }

    /// PUT /api/config/{guild_id}/{bot_name} — memorise une valeur de config.
    ///
    /// Utilise notamment pour persister l'ID de la categorie de sessions creee
    /// automatiquement au premier demarrage, afin de ne plus la rechercher.
    pub async fn set_config(
        &self,
        guild_id: &str,
        bot_name: &str,
        key: &str,
        value: &str,
    ) -> Result<(), String> {
        let url = format!(
            "{}/api/config/{}/{}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(bot_name)
        );
        let body = serde_json::json!({ "key": key, "value": value });
        let _: serde_json::Value = self.send(self.http.put(&url).json(&body)).await?;
        Ok(())
    }

    /// PATCH /api/games/servers/{server_id}/session-channels.
    ///
    /// Renvoie `claimed` : `false` signifie que des salons etaient deja
    /// enregistres (evenement rejoue) — l'appelant doit supprimer ceux qu'il
    /// vient de creer en double.
    pub async fn set_session_channels(
        &self,
        server_id: &str,
        text_channel_id: Option<&str>,
        voice_channel_id: Option<&str>,
    ) -> Result<bool, String> {
        let url = format!(
            "{}/api/games/servers/{}/session-channels",
            self.base_url,
            encode_segment(server_id)
        );
        let body = serde_json::json!({
            "text_channel_id": text_channel_id,
            "voice_channel_id": voice_channel_id,
        });
        let v: serde_json::Value = self.send(self.http.patch(&url).json(&body)).await?;
        Ok(v.get("claimed").and_then(|c| c.as_bool()).unwrap_or(true))
    }

    /// GET /api/wallet/{guild_id}/{user_id}.
    pub async fn get_wallet(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<WalletResponse, String> {
        let url = format!("{}/api/wallet/{guild_id}/{user_id}", self.base_url);
        self.send(self.http.get(&url)).await
    }

    /// POST /api/wallet/{guild_id}/transfer.
    pub async fn transfer_coins(
        &self,
        guild_id: &str,
        req: &TransferRequest,
    ) -> Result<TransferResponse, String> {
        let url = format!("{}/api/wallet/{guild_id}/transfer", self.base_url);
        self.send(self.http.post(&url).json(req)).await
    }

    /// GET /api/wallet/{guild_id}/leaderboard?limit=N.
    pub async fn wallet_leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<WalletResponse>, String> {
        let url = format!(
            "{}/api/wallet/{guild_id}/leaderboard?limit={limit}",
            self.base_url
        );
        self.send(self.http.get(&url)).await
    }

    /// POST /api/wheel/{guild_id}/{user_id}/spin.
    /// Err(message affichable) sur 4xx (ex: daily deja claim) ou erreur reseau.
    pub async fn spin_wheel(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<WheelSpinResponse, String> {
        let url = format!("{}/api/wheel/{guild_id}/{user_id}/spin", self.base_url);
        let mut req = self.http.post(&url).json(&WheelSpinRequest {
            username: username.to_string(),
        });
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("nexus-api injoignable: {e}"))?;

        let status = resp.status();
        if status.is_success() {
            resp.json::<WheelSpinResponse>()
                .await
                .map_err(|e| format!("reponse nexus-api invalide: {e}"))
        } else {
            let msg = resp
                .json::<ApiErrorBody>()
                .await
                .map(|b| b.error)
                .unwrap_or_else(|_| format!("erreur nexus-api ({status})"));
            Err(msg)
        }
    }

    pub async fn challenge_coussin(
        &self,
        guild_id: &str,
        body: &CoussinChallengeRequest,
    ) -> Result<CoussinChallengeResponse, String> {
        let url = format!(
            "{}/api/coussin/{}/combats",
            self.base_url,
            encode_segment(guild_id)
        );
        self.send(self.http.post(url).json(body)).await
    }

    pub async fn accept_coussin(&self, id: &str, defender_id: &str) -> Result<bool, String> {
        let url = format!(
            "{}/api/coussin/combats/{}/accept",
            self.base_url,
            encode_segment(id)
        );
        let response: serde_json::Value = self
            .send(self.http.post(url).json(&CoussinDefenderRequest {
                defender_id: defender_id.into(),
            }))
            .await?;
        response["ok"]
            .as_bool()
            .ok_or_else(|| "reponse nexus-api invalide".into())
    }

    pub async fn refuse_coussin(&self, id: &str, defender_id: &str) -> Result<bool, String> {
        let url = format!(
            "{}/api/coussin/combats/{}/refuse",
            self.base_url,
            encode_segment(id)
        );
        let response: serde_json::Value = self
            .send(self.http.post(url).json(&CoussinDefenderRequest {
                defender_id: defender_id.into(),
            }))
            .await?;
        response["ok"]
            .as_bool()
            .ok_or_else(|| "reponse nexus-api invalide".into())
    }

    pub async fn resolve_coussin(&self, id: &str) -> Result<bool, String> {
        let url = format!(
            "{}/api/coussin/combats/{}/resolve",
            self.base_url,
            encode_segment(id)
        );
        let response: serde_json::Value = self.send(self.http.post(url)).await?;
        response["ok"]
            .as_bool()
            .ok_or_else(|| "reponse nexus-api invalide".into())
    }

    pub async fn coussin_profile(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<CoussinProfileResponse, String> {
        let url = format!(
            "{}/api/coussin/{}/{}/profile?username={}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id),
            encode_segment(username)
        );
        self.send(self.http.get(url)).await
    }
    pub async fn choose_coussin_class(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        class: &str,
    ) -> Result<CoussinProfileResponse, String> {
        let url = format!(
            "{}/api/coussin/{}/{}/class",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id)
        );
        self.send(self.http.post(url).json(&CoussinClassRequest {
            username: username.into(),
            class: class.into(),
        }))
        .await
    }
    pub async fn train_coussin(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        stat: &str,
    ) -> Result<CoussinProfileResponse, String> {
        let url = format!(
            "{}/api/coussin/{}/{}/train",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id)
        );
        self.send(self.http.post(url).json(&CoussinTrainRequest {
            username: username.into(),
            stat: stat.into(),
        }))
        .await
    }
    pub async fn buy_coussin_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<i64, String> {
        let url = format!(
            "{}/api/coussin/{}/{}/shop",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id)
        );
        let value: serde_json::Value = self
            .send(self.http.post(url).json(&CoussinBuyItemRequest {
                item_key: item_key.into(),
            }))
            .await?;
        value["balance_after"]
            .as_i64()
            .ok_or_else(|| "reponse nexus-api invalide".into())
    }
    pub async fn buy_coussin_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(bool, String), String> {
        let url = format!(
            "{}/api/coussin/{}/{}/insurance",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id)
        );
        let value: serde_json::Value = self.send(self.http.post(url)).await?;
        Ok((
            value["is_scam"]
                .as_bool()
                .ok_or_else(|| "reponse nexus-api invalide".to_string())?,
            value["expires_at"].as_str().unwrap_or("").to_string(),
        ))
    }
    pub async fn steal_coussin(
        &self,
        guild: &str,
        user: &str,
        body: &CoussinStealRequest,
    ) -> Result<(bool, i64), String> {
        let url = format!(
            "{}/api/coussin/{}/{}/steal",
            self.base_url,
            encode_segment(guild),
            encode_segment(user)
        );
        let v: serde_json::Value = self.send(self.http.post(url).json(body)).await?;
        Ok((
            v["success"].as_bool().unwrap_or(false),
            v["amount"].as_i64().unwrap_or(0),
        ))
    }
    pub async fn prime_coussin(
        &self,
        guild: &str,
        user: &str,
        body: &CoussinPrimeRequest,
    ) -> Result<(), String> {
        let url = format!(
            "{}/api/coussin/{}/{}/prime",
            self.base_url,
            encode_segment(guild),
            encode_segment(user)
        );
        let _: serde_json::Value = self.send(self.http.post(url).json(body)).await?;
        Ok(())
    }
    pub async fn inventory_coussin(
        &self,
        guild: &str,
        user: &str,
    ) -> Result<Vec<CoussinInventoryItem>, String> {
        let url = format!(
            "{}/api/coussin/{}/{}/inventory",
            self.base_url,
            encode_segment(guild),
            encode_segment(user)
        );
        self.send(self.http.get(url)).await
    }
    pub async fn bet_coussin(
        &self,
        guild: &str,
        user: &str,
        body: &CoussinBetRequest,
    ) -> Result<(), String> {
        let url = format!(
            "{}/api/coussin/{}/{}/bets",
            self.base_url,
            encode_segment(guild),
            encode_segment(user)
        );
        let _: serde_json::Value = self.send(self.http.post(url).json(body)).await?;
        Ok(())
    }
}
