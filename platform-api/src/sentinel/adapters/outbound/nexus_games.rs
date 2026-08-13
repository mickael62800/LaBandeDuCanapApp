//! Client vers nexus-api pour les jeux joues depuis le site.
//!
//! # Pourquoi passer par sentinel-api et non appeler nexus-api directement
//!
//! Les routes de nexus-api portent le joueur dans leur CHEMIN :
//! `/api/wheel/{guild_id}/{user_id}/spin`, `/api/wallet/{guild_id}/transfer`.
//! Exposees au navigateur, n'importe qui tirerait la roue a la place d'un
//! autre — ou viderait son portefeuille.
//!
//! sentinel-api est le seul composant qui sait QUI est connecte : il derive
//! le `user_id` de la session Discord et ne le lit jamais de la requete. Ce
//! client est donc volontairement le seul chemin d'acces aux jeux depuis le
//! web, et chacune de ses methodes recoit l'identifiant du joueur depuis
//! l'appelant, jamais depuis un corps de requete.
//!
//! # Le portefeuille reste unique
//!
//! Aucune logique de jeu ici : ce client relaie vers les memes endpoints que
//! le bot Discord appelle. Le quota quotidien de la Roue
//! (`try_claim_today`, atomique) et les mouvements de coins vivent dans
//! nexus-core, une seule fois. Tirer sur le site consomme donc le tirage du
//! jour sur Discord, et le solde est le meme des deux cotes — non par
//! synchronisation, mais parce qu'il n'existe qu'un seul portefeuille.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use platform_core::sentinel::domain::errors::DomainError;

/// Delai au-dela duquel on renonce. Court : ces appels sont declenches par un
/// clic, un utilisateur qui attend dix secondes reclique et croit a un bug.
const TIMEOUT_SECS: u64 = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub amount: i64,
    pub balance_after: i64,
    pub source: String,
    pub description: String,
    pub reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelStatus {
    pub can_spin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpinResult {
    pub spin_id: String,
    pub case_key: String,
    pub case_label: String,
    pub payout: i64,
    pub balance_after: i64,
    pub is_memorable: bool,
}

/// Une case de la roue, telle que le serveur l'a definie.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WheelCase {
    pub key: String,
    pub label: String,
    pub payout: i64,
    pub weight: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WheelCases {
    pub cases: Vec<WheelCase>,
    pub customized: bool,
}

/// Profil Coussin Piégé : classe, statistiques, palmares.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoussinProfile {
    pub username: String,
    pub class: String,
    pub level: i32,
    pub xp: i32,
    pub atk: i32,
    pub def: i32,
    pub hp_current: i32,
    pub hp_max: i32,
    pub coins: i64,
    pub stat_points: i32,
    pub title: Option<String>,
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub chaos_events: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoussinItem {
    pub item_key: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoussinCombat {
    pub id: String,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub special_attack: Option<String>,
    pub result_message: Option<String>,
    pub coins_transferred: i64,
    pub resolved_at: Option<String>,
}

pub struct NexusGamesClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl NexusGamesClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(TIMEOUT_SECS))
                .build()
                .unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    /// Configure ? Sans URL, la passerelle repond « jeux indisponibles »
    /// plutot que d'echouer sur une requete vers une adresse vide.
    pub fn is_configured(&self) -> bool {
        !self.base_url.is_empty()
    }

    fn url(&self, chemin: &str) -> String {
        format!("{}{}", self.base_url, chemin)
    }

    /// Envoie la requete et traduit la reponse.
    ///
    /// Les erreurs METIER de nexus-api (« deja tire aujourd'hui », « solde
    /// insuffisant ») arrivent en 4xx avec un message destine au joueur : on
    /// le fait remonter tel quel. Les 5xx et les pannes reseau deviennent un
    /// message generique — l'utilisateur n'a que faire d'un detail
    /// d'infrastructure.
    async fn envoyer<T: for<'de> Deserialize<'de>>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T, DomainError> {
        let reponse = req.bearer_auth(&self.api_key).send().await.map_err(|e| {
            tracing::warn!(error = %e, "nexus-games : appel echoue");
            DomainError::Internal("plateforme de jeux injoignable".into())
        })?;

        let statut = reponse.status();
        if statut.is_success() {
            return reponse.json::<T>().await.map_err(|e| {
                tracing::warn!(error = %e, "nexus-games : reponse illisible");
                DomainError::Internal("reponse inattendue de la plateforme de jeux".into())
            });
        }

        let corps = reponse.text().await.unwrap_or_default();
        if statut.is_client_error() {
            // Le corps est du JSON `{"error": "..."}` ou du texte brut selon
            // le handler : on tente le premier, on retombe sur le second.
            let message = serde_json::from_str::<serde_json::Value>(&corps)
                .ok()
                .and_then(|v| {
                    v.get("error")
                        .or_else(|| v.get("message"))
                        .and_then(|m| m.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| corps.clone());

            let message = if message.trim().is_empty() {
                "action refusee par la plateforme de jeux".to_string()
            } else {
                message
            };
            return Err(DomainError::ValidationError(message));
        }

        tracing::warn!(status = %statut, corps = %corps, "nexus-games : erreur serveur");
        Err(DomainError::Internal(
            "la plateforme de jeux a rencontre une erreur".into(),
        ))
    }

    /// Tire la Roue du Destin. Un tirage par jour et par personne, arbitre
    /// par nexus-core — pas ici.
    pub async fn spin_wheel(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<SpinResult, DomainError> {
        let url = self.url(&format!("/api/wheel/{guild_id}/{user_id}/spin"));
        self.envoyer(
            self.client
                .post(url)
                .json(&serde_json::json!({ "username": username })),
        )
        .await
    }

    /// Les cases de la roue du serveur.
    ///
    /// Le site les DESSINE : sans cet appel il afficherait les dix cases
    /// d'origine a un serveur qui a change les siennes, et la fleche
    /// s'arreterait sur un secteur qui n'est pas celui tire.
    pub async fn wheel_cases(&self, guild_id: &str) -> Result<WheelCases, DomainError> {
        let url = self.url(&format!("/api/wheel/{guild_id}/cases"));
        self.envoyer(self.client.get(url)).await
    }

    /// Le joueur peut-il encore tirer aujourd'hui ? Lecture seule.
    pub async fn wheel_status(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<WheelStatus, DomainError> {
        let url = self.url(&format!("/api/wheel/{guild_id}/{user_id}/status"));
        self.envoyer(self.client.get(url)).await
    }

    pub async fn wallet(&self, guild_id: &str, user_id: &str) -> Result<Wallet, DomainError> {
        let url = self.url(&format!("/api/wallet/{guild_id}/{user_id}"));
        self.envoyer(self.client.get(url)).await
    }

    pub async fn history(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<Transaction>, DomainError> {
        let url = self.url(&format!(
            "/api/wallet/{guild_id}/{user_id}/history?limit={limit}"
        ));
        self.envoyer(self.client.get(url)).await
    }

    // ── Coussin Piégé ──
    //
    // Lectures SEULEMENT. Les actions du jeu — coussin piégé, vol, prime,
    // pari — restent sur Discord : leur interet tient a la reaction dans le
    // salon, et les ouvrir au web viderait la conversation de ce qui la fait
    // vivre.

    /// Profil du joueur. `username` sert a creer le profil au premier appel :
    /// le jeu inscrit un joueur des sa premiere consultation.
    pub async fn coussin_profile(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<CoussinProfile, DomainError> {
        let url = self.url(&format!(
            "/api/coussin/{guild_id}/{user_id}/profile?username={}",
            urlencoding(username)
        ));
        self.envoyer(self.client.get(url)).await
    }

    pub async fn coussin_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<CoussinItem>, DomainError> {
        let url = self.url(&format!("/api/coussin/{guild_id}/{user_id}/inventory"));
        self.envoyer(self.client.get(url)).await
    }

    pub async fn coussin_combats(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CoussinCombat>, DomainError> {
        let url = self.url(&format!(
            "/api/coussin/{guild_id}/{user_id}/combats?limit={limit}"
        ));
        self.envoyer(self.client.get(url)).await
    }

    pub async fn coussin_ranking(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<CoussinProfile>, DomainError> {
        let url = self.url(&format!("/api/coussin/{guild_id}/classement?limit={limit}"));
        self.envoyer(self.client.get(url)).await
    }

    pub async fn leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<Wallet>, DomainError> {
        let url = self.url(&format!("/api/wallet/{guild_id}/leaderboard?limit={limit}"));
        self.envoyer(self.client.get(url)).await
    }
}

/// Encodage minimal pour un parametre de requete.
///
/// Les pseudos Discord contiennent espaces, accents et emojis : les laisser
/// bruts casserait l'URL. Une dependance dediee serait disproportionnee pour
/// le seul parametre qu'on transmet.
fn urlencoding(v: &str) -> String {
    v.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
