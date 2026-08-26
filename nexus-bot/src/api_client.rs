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
    /// Ce que la session annonce : `waiting` | `open` | `closed`.
    ///
    /// Calcule par l'API a partir de la fenetre horaire ET du conteneur. Le
    /// bot ne le recalcule pas : Discord et le site racontaient la meme
    /// session differemment, chacun avec sa propre regle.
    ///
    /// Absent d'une reponse ancienne : on retombe alors sur le statut brut.
    #[serde(default)]
    pub display_state: Option<String>,
    /// Instant de publication de l'annonce Atrium. `None` = pas encore
    /// publiee : le bot doit la demander avant de poser le panneau.
    #[serde(default)]
    pub announcement_posted_at: Option<String>,
    #[serde(default)]
    pub channel_name_registration: Option<String>,
    #[serde(default)]
    pub channel_name_private: Option<String>,
    #[serde(default)]
    pub channel_name_voice: Option<String>,
    pub text_channel_id: Option<String>,
    pub voice_channel_id: Option<String>,
    /// Derniers joueurs vus par la console du jeu. Zero quand cette console
    /// n'est pas lisible (la plupart des jeux) : ce n'est donc PAS une preuve
    /// de serveur vide. Cf. `compteurs`.
    #[serde(default)]
    pub last_player_count: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameTemplate {
    pub slug: String,
    pub name: String,
    pub cover_image_url: Option<String>,
    /// Description des reglages du jeu. Sert a nommer les parametres en
    /// francais : sans elle, `/game parametres` afficherait `SPAWN_MONSTERS`,
    /// ce qui ne dit rien a un joueur.
    #[serde(default)]
    pub config_schema: Vec<TemplateField>,
}

/// Un reglage du jeu, tel que le decrit son modele.
#[derive(Debug, Clone, Deserialize)]
pub struct TemplateField {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub group: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_new() {
        let client = ApiClient::new("http://localhost:3100/".into(), Some("my_key".into()));
        assert_eq!(client.base_url, "http://localhost:3100");
        assert_eq!(client.api_key.as_deref(), Some("my_key"));

        let client2 = ApiClient::new("http://nexus-api:3100".into(), None);
        assert_eq!(client2.base_url, "http://nexus-api:3100");
        assert_eq!(client2.api_key, None);
    }

    #[test]
    fn test_encode_segment() {
        assert_eq!(encode_segment("simple"), "simple");
        assert_eq!(encode_segment("hello world"), "hello%20world");
        assert_eq!(encode_segment("a/b"), "a%2Fb");
    }

    #[test]
    fn test_deserialize_wheel_and_wallet() {
        let wheel_json =
            r#"{"case_label":"Jackpot","payout":1000,"balance_after":1500,"is_memorable":true}"#;
        let wheel: WheelSpinResponse = serde_json::from_str(wheel_json).unwrap();
        assert_eq!(wheel.case_label, "Jackpot");
        assert_eq!(wheel.payout, 1000);
        assert!(wheel.is_memorable);

        let wallet_json = r#"{"user_id":"u1","coins":500,"total_earned":1000,"total_spent":500}"#;
        let wallet: WalletResponse = serde_json::from_str(wallet_json).unwrap();
        assert_eq!(wallet.user_id, "u1");
        assert_eq!(wallet.coins, 500);

        let trans_json = r#"{"amount":100,"from_balance":400}"#;
        let trans: TransferResponse = serde_json::from_str(trans_json).unwrap();
        assert_eq!(trans.amount, 100);
    }

    #[test]
    fn test_serialize_requests() {
        let wheel_req = WheelSpinRequest {
            username: "Alice".into(),
        };
        let json = serde_json::to_string(&wheel_req).unwrap();
        assert!(json.contains("Alice"));

        let trans_req = TransferRequest {
            from_user_id: "u1".into(),
            from_username: "Alice".into(),
            to_user_id: "u2".into(),
            to_username: "Bob".into(),
            amount: 50,
            reason: Some("gift".into()),
        };
        let json_trans = serde_json::to_string(&trans_req).unwrap();
        assert!(json_trans.contains("gift"));

        let join_req = GrandSalonJoinRequest {
            display_name: "Alice".into(),
        };
        assert!(serde_json::to_string(&join_req).unwrap().contains("Alice"));

        let challenge_req = CoussinChallengeRequest {
            channel_id: "c1".into(),
            attacker_id: "a1".into(),
            attacker_name: "Att".into(),
            defender_id: "d1".into(),
            defender_name: "Def".into(),
            mise: 100,
        };
        assert!(serde_json::to_string(&challenge_req)
            .unwrap()
            .contains("100"));

        let def_req = CoussinDefenderRequest {
            defender_id: "d1".into(),
        };
        assert!(serde_json::to_string(&def_req).unwrap().contains("d1"));

        let class_req = CoussinClassRequest {
            username: "Alice".into(),
            class: "guerrier".into(),
        };
        assert!(serde_json::to_string(&class_req)
            .unwrap()
            .contains("guerrier"));

        let train_req = CoussinTrainRequest {
            username: "Alice".into(),
            stat: "atk".into(),
        };
        assert!(serde_json::to_string(&train_req).unwrap().contains("atk"));

        let buy_req = CoussinBuyItemRequest {
            item_key: "potion".into(),
        };
        assert!(serde_json::to_string(&buy_req).unwrap().contains("potion"));

        let steal_req = CoussinStealRequest {
            thief_name: "Thief".into(),
            victim_id: "v1".into(),
            victim_name: "Vic".into(),
            channel_id: "ch1".into(),
        };
        assert!(serde_json::to_string(&steal_req).unwrap().contains("Thief"));

        let prime_req = CoussinPrimeRequest {
            target_id: "t1".into(),
            target_name: "Target".into(),
            placer_name: "Placer".into(),
            amount: 200,
        };
        assert!(serde_json::to_string(&prime_req).unwrap().contains("200"));

        let bet_req = CoussinBetRequest {
            combat_id: "cb1".into(),
            bettor_name: "Bettor".into(),
            backed_id: "b1".into(),
            amount: 50,
        };
        assert!(serde_json::to_string(&bet_req).unwrap().contains("50"));
    }

    #[test]
    fn test_deserialize_coussin_and_salon() {
        let salon_json = r#"{"display_name":"Alice","rayonnement":10,"jetons":5,"reputation":20,"bons_plans":2,"reseau":3}"#;
        let salon: GrandSalonProfileResponse = serde_json::from_str(salon_json).unwrap();
        assert_eq!(salon.display_name, "Alice");
        assert_eq!(salon.rayonnement, 10);

        let ch_resp_json = r#"{"id":"ch_1","mise":100}"#;
        let ch_resp: CoussinChallengeResponse = serde_json::from_str(ch_resp_json).unwrap();
        assert_eq!(ch_resp.id, "ch_1");

        let prof_json = r#"{
            "username":"Alice","class":"Voleur","level":5,"xp":1000,"atk":15,"def":10,
            "hp_current":50,"hp_max":50,"coins":200,"stat_points":3,"title":"L'Agile",
            "total_wins":10,"total_losses":2,"total_draws":1,"total_stolen":500,
            "cowardice_count":0,"chaos_events":1
        }"#;
        let prof: CoussinProfileResponse = serde_json::from_str(prof_json).unwrap();
        assert_eq!(prof.class, "Voleur");
        assert_eq!(prof.level, 5);

        let steal_opened_json = r#"{"attempt_id":"att_1","defense_window_seconds":30}"#;
        let steal_opened: CoussinStealOpened = serde_json::from_str(steal_opened_json).unwrap();
        assert_eq!(steal_opened.attempt_id, "att_1");

        let steal_out_json =
            r#"{"thief_id":"th1","success":true,"amount":100,"thief_total":15,"victim_total":10}"#;
        let steal_out: CoussinStealOutcome = serde_json::from_str(steal_out_json).unwrap();
        assert!(steal_out.success);

        let inv_json = r#"{"item_key":"shield","quantity":2}"#;
        let inv: CoussinInventoryItem = serde_json::from_str(inv_json).unwrap();
        assert_eq!(inv.item_key, "shield");
    }

    #[test]
    fn test_deserialize_games_and_server_types() {
        let game_json = r#"{"id":"g1","game_name":"Minecraft","emoji":"⛏️","category":"Survie","role_id":"r1"}"#;
        let g: Game = serde_json::from_str(game_json).unwrap();
        assert_eq!(g.game_name, "Minecraft");

        let panel_json = r#"{"id":"p1","channel_id":"c1","message_id":"m1","category":"Survie"}"#;
        let p: GamePanel = serde_json::from_str(panel_json).unwrap();
        assert_eq!(p.id, "p1");

        let server_json = r#"{
            "id":"s1","guild_id":"g1","template_id":"t1","name":"Mon Serveur",
            "status":"Running","owner_user_id":"u1","host_port":25565,
            "public_host":"play.com","ip_reveal_at":"2026-01-01T00:00:00Z","ip_revealed":true,
            "display_state":"open","text_channel_id":"tc1","voice_channel_id":"vc1",
            "last_player_count":3
        }"#;
        let s: GameServer = serde_json::from_str(server_json).unwrap();
        assert_eq!(s.name, "Mon Serveur");
        assert_eq!(s.last_player_count, 3);

        let server_detail_json =
            format!(r#"{{"server":{server_json},"config":{{"MOTD":"Bienvenue"}}}}"#);
        let sd: ServerDetailResponse = serde_json::from_str(&server_detail_json).unwrap();
        assert_eq!(sd.server.id, "s1");
        assert_eq!(sd.config.get("MOTD").map(|s| s.as_str()), Some("Bienvenue"));

        let template_json = r#"{
            "slug":"mc","name":"Minecraft","cover_image_url":"http://img.png",
            "config_schema":[{"key":"PVP","label":"Joueur contre Joueur","group":"Gameplay"}]
        }"#;
        let tmpl: GameTemplate = serde_json::from_str(template_json).unwrap();
        assert_eq!(tmpl.slug, "mc");
        assert_eq!(tmpl.config_schema[0].label, "Joueur contre Joueur");

        let tsettings_json = r#"{"template_slug":"mc","discord_role_id":"r1"}"#;
        let ts: TemplateSettings = serde_json::from_str(tsettings_json).unwrap();
        assert_eq!(ts.template_slug, "mc");

        let sreg_json = r#"{"user_id":"u1"}"#;
        let sreg: ServerRegistration = serde_json::from_str(sreg_json).unwrap();
        assert_eq!(sreg.user_id, "u1");

        let rev_json = r#"{"delay_minutes":10,"started":true}"#;
        let rev: RevealRequest = serde_json::from_str(rev_json).unwrap();
        assert_eq!(rev.delay_minutes, 10);
        assert!(rev.started);

        let ach_json = r#"{"name":"Vainqueur","description":"Gagne un combat","icon_url":"http://icon.png","unlocked_at":"2026-01-01"}"#;
        let ach: AchievementProgress = serde_json::from_str(ach_json).unwrap();
        assert_eq!(ach.name, "Vainqueur");

        let err_json = r#"{"error":"Solde insuffisant"}"#;
        let err: ApiErrorBody = serde_json::from_str(err_json).unwrap();
        assert_eq!(err.error, "Solde insuffisant");
    }

    #[tokio::test]
    async fn test_all_api_client_http_endpoints() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req_str = String::from_utf8_lossy(&buf[..n]);

                let body = if req_str.contains("/api/grand-salon/") {
                    r#"{"display_name":"Alice","rayonnement":10,"jetons":5,"reputation":20,"bons_plans":2,"reseau":3}"#
                } else if req_str.contains("/api/wallet/") && req_str.contains("/transfer") {
                    r#"{"amount":50,"from_balance":450}"#
                } else if req_str.contains("/api/wallet/") && req_str.contains("/leaderboard") {
                    r#"[{"user_id":"u1","coins":500,"total_earned":1000,"total_spent":500}]"#
                } else if req_str.contains("/api/wallet/") {
                    r#"{"user_id":"u1","coins":500,"total_earned":1000,"total_spent":500}"#
                } else if req_str.contains("/api/wheel/") {
                    r#"{"case_label":"Jackpot","payout":1000,"balance_after":1500,"is_memorable":true}"#
                } else if req_str.contains("/api/achievements/") {
                    r#"[{"name":"Vainqueur","description":"Gagne","icon_url":null,"unlocked_at":null}]"#
                } else if req_str.contains("/api/coussin/combats/") {
                    r#"{"ok":true}"#
                } else if req_str.contains("/api/coussin/") && req_str.contains("/combats") {
                    r#"{"id":"cb_1","mise":100}"#
                } else if req_str.contains("/api/coussin/") && req_str.contains("/profile") {
                    r#"{"username":"Alice","class":"Voleur","level":1,"xp":0,"atk":10,"def":10,"hp_current":30,"hp_max":30,"coins":100,"stat_points":0,"title":"Novice","total_wins":0,"total_losses":0,"total_draws":0,"total_stolen":0,"cowardice_count":0,"chaos_events":0}"#
                } else if req_str.contains("/api/coussin/") && req_str.contains("/class") {
                    r#"{"username":"Alice","class":"Guerrier","level":1,"xp":0,"atk":10,"def":10,"hp_current":30,"hp_max":30,"coins":100,"stat_points":0,"title":"Novice","total_wins":0,"total_losses":0,"total_draws":0,"total_stolen":0,"cowardice_count":0,"chaos_events":0}"#
                } else if req_str.contains("/api/coussin/") && req_str.contains("/train") {
                    r#"{"username":"Alice","class":"Guerrier","level":1,"xp":0,"atk":12,"def":10,"hp_current":30,"hp_max":30,"coins":100,"stat_points":0,"title":"Novice","total_wins":0,"total_losses":0,"total_draws":0,"total_stolen":0,"cowardice_count":0,"chaos_events":0}"#
                } else if req_str.contains("/api/coussin/") && req_str.contains("/shop") {
                    r#"{"balance_after":80}"#
                } else if req_str.contains("/api/coussin/") && req_str.contains("/insurance") {
                    r#"{"is_scam":false,"expires_at":"2026-12-31"}"#
                } else if req_str.contains("/api/coussin/steals/") && req_str.contains("/defend/") {
                    r#"{"thief_id":"t1","success":false,"amount":10,"thief_total":5,"victim_total":12}"#
                } else if req_str.contains("/api/coussin/") && req_str.contains("/steal") {
                    r#"{"attempt_id":"att_1","defense_window_seconds":30}"#
                } else if req_str.contains("/api/coussin/") && req_str.contains("/inventory") {
                    r#"[{"item_key":"shield","quantity":1}]"#
                } else if req_str.contains("/api/games/servers/")
                    && req_str.contains("/reveal-ip/request")
                {
                    r#"{"delay_minutes":5,"started":true}"#
                } else if req_str.contains("/api/games/servers/")
                    && req_str.contains("/registrations")
                {
                    r#"[{"user_id":"u1"}]"#
                } else if req_str.contains("/api/games/servers/")
                    && req_str.contains("/session-channels")
                {
                    r#"{"claimed":true}"#
                } else if req_str.contains("/api/games/servers/") {
                    r#"{"server":{"id":"s1","guild_id":"g1","template_id":"t1","name":"Server","status":"Running","owner_user_id":"u1","host_port":25565,"public_host":"host","ip_reveal_at":null,"ip_revealed":true,"display_state":"open","text_channel_id":null,"voice_channel_id":null,"last_player_count":0},"config":{}}"#
                } else if req_str.contains("/api/games/templates/") {
                    r#"{"slug":"valheim","name":"Valheim","cover_image_url":null,"config_schema":[]}"#
                } else if req_str.contains("/api/games/") && req_str.contains("/template-settings")
                {
                    r#"[{"template_slug":"valheim","discord_role_id":"123"}]"#
                } else if req_str.contains("/api/games/") && req_str.contains("/servers") {
                    r#"[{"id":"s1","guild_id":"g1","template_id":"t1","name":"Server","status":"Running","owner_user_id":"u1","host_port":25565,"public_host":"host","ip_reveal_at":null,"ip_revealed":true,"display_state":"open","text_channel_id":null,"voice_channel_id":null,"last_player_count":0}]"#
                } else if req_str.contains("/api/games/") && req_str.contains("/panels") {
                    if req_str.starts_with("POST ") || req_str.contains("/panels/") {
                        r#"{"id":"p1","channel_id":"c1","message_id":"m1","category":null}"#
                    } else {
                        r#"[{"id":"p1","channel_id":"c1","message_id":"m1","category":null}]"#
                    }
                } else if req_str.contains("/api/games/") && req_str.contains("/by-name/") {
                    r#"{"id":"g1","game_name":"Minecraft","emoji":"⛏️","category":null,"role_id":null}"#
                } else if req_str.contains("/api/games/") && req_str.contains("/by-category") {
                    r#"[{"id":"g1","game_name":"Minecraft","emoji":"⛏️","category":null,"role_id":null}]"#
                } else if req_str.contains("/api/games/") && req_str.contains("/role") {
                    r#"{"id":"g1","game_name":"Minecraft","emoji":"⛏️","category":null,"role_id":"123"}"#
                } else if req_str.starts_with("POST /api/games") {
                    r#"{"id":"g1","game_name":"Minecraft","emoji":"⛏️","category":null,"role_id":null}"#
                } else if req_str.contains("/api/games") {
                    r#"[{"id":"g1","game_name":"Minecraft","emoji":"⛏️","category":null,"role_id":null}]"#
                } else if req_str.contains("/api/config/") {
                    r#"{"session_category_id":"123456"}"#
                } else {
                    r#"{"ok":true}"#
                };

                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            }
        });

        let client = ApiClient::new(base_url, Some("secret_api_key".into()));

        // Economy
        assert!(client.grand_salon_join("g1", "u1", "Alice").await.is_ok());
        assert!(client.grand_salon_profile("g1", "u1").await.is_ok());
        assert!(client.get_wallet("g1", "u1").await.is_ok());
        assert!(client
            .transfer_coins(
                "g1",
                &TransferRequest {
                    from_user_id: "u1".into(),
                    from_username: "Alice".into(),
                    to_user_id: "u2".into(),
                    to_username: "Bob".into(),
                    amount: 50,
                    reason: None,
                }
            )
            .await
            .is_ok());
        assert!(client.wallet_leaderboard("g1", 10).await.is_ok());
        assert!(client.spin_wheel("g1", "u1", "Alice").await.is_ok());

        // Achievements
        assert!(client
            .member_achievements("g1", "u1", Some("valheim"))
            .await
            .is_ok());

        // Coussin
        assert!(client
            .challenge_coussin(
                "g1",
                &CoussinChallengeRequest {
                    channel_id: "c1".into(),
                    attacker_id: "a1".into(),
                    attacker_name: "Att".into(),
                    defender_id: "d1".into(),
                    defender_name: "Def".into(),
                    mise: 100,
                }
            )
            .await
            .is_ok());
        assert!(client.accept_coussin("cb_1", "d1").await.unwrap());
        assert!(client.refuse_coussin("cb_1", "d1").await.unwrap());
        assert!(client.resolve_coussin("cb_1").await.unwrap());
        assert!(client.coussin_profile("g1", "u1", "Alice").await.is_ok());
        assert!(client
            .choose_coussin_class("g1", "u1", "Alice", "guerrier")
            .await
            .is_ok());
        assert!(client
            .train_coussin("g1", "u1", "Alice", "atk")
            .await
            .is_ok());
        assert_eq!(
            client.buy_coussin_item("g1", "u1", "potion").await.unwrap(),
            80
        );
        assert!(!client.buy_coussin_insurance("g1", "u1").await.unwrap().0);
        assert!(client
            .steal_coussin(
                "g1",
                "u1",
                &CoussinStealRequest {
                    thief_name: "Thief".into(),
                    victim_id: "v1".into(),
                    victim_name: "Vic".into(),
                    channel_id: "c1".into(),
                }
            )
            .await
            .is_ok());
        assert!(client.attach_steal_message("att_1", "msg_1").await.is_ok());
        assert!(client.defend_steal("att_1", "v1").await.is_ok());
        assert!(client
            .prime_coussin(
                "g1",
                "u1",
                &CoussinPrimeRequest {
                    target_id: "t1".into(),
                    target_name: "T".into(),
                    placer_name: "P".into(),
                    amount: 50,
                }
            )
            .await
            .is_ok());
        assert!(client.inventory_coussin("g1", "u1").await.is_ok());
        assert!(client
            .bet_coussin(
                "g1",
                "u1",
                &CoussinBetRequest {
                    combat_id: "cb_1".into(),
                    bettor_name: "B".into(),
                    backed_id: "u2".into(),
                    amount: 20,
                }
            )
            .await
            .is_ok());

        // Games
        assert!(client.list_games("g1").await.is_ok());
        assert!(client
            .list_games_by_category("g1", Some("RPG"))
            .await
            .is_ok());
        assert!(client
            .create_game(
                "g1",
                "Valheim",
                "u1",
                Some("r1"),
                Some("⚔️"),
                Some("Survie")
            )
            .await
            .is_ok());
        assert!(client.set_game_role("g1", "g1", "r1").await.is_ok());
        assert!(client.delete_game("g1", "g1").await.is_ok());
        assert!(client.get_game_by_name("g1", "Minecraft").await.is_ok());
        assert!(client
            .save_panel("g1", "c1", "m1", Some("Survie"))
            .await
            .is_ok());
        assert!(client.list_panels("g1").await.is_ok());
        assert!(client.find_panel_by_message("g1", "m1").await.is_ok());
        assert!(client
            .put_sync_inventory("g1", &serde_json::json!({}))
            .await
            .is_ok());
        assert!(client.report_vanished_role("g1", "r1").await.is_ok());

        // Game portal
        assert!(client.list_game_servers("g1").await.is_ok());
        assert!(client.get_game_server("s1").await.is_ok());
        assert!(client.get_game_template("t1").await.is_ok());
        assert!(client.register_to_server("s1", "u1").await.is_ok());
        assert!(client.unregister_from_server("s1", "u1").await.is_ok());
        assert!(client.request_reveal_ip("s1", "u1").await.is_ok());
        assert!(client.list_server_registrations("s1").await.is_ok());
        assert!(client.list_template_settings("g1").await.is_ok());
        assert!(client.get_guild_config("g1", "game-portal").await.is_ok());
        assert!(client
            .set_config("g1", "game-portal", "k", "v")
            .await
            .is_ok());
        assert!(client
            .set_session_channels("s1", Some("c1"), Some("c2"))
            .await
            .unwrap());
    }
}
