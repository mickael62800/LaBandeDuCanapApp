use async_trait::async_trait;

use sentinel_core::domain::errors::DomainError;
pub use sentinel_core::ports::outbound::discord_api::{
    DiscordApi, DiscordChannel, DiscordEmoji, DiscordMember, DiscordRoleInfo, DiscordUser, NewChannel, UserGuild,
};

/// Service pour les appels a l'API Discord.
/// Centralise la logique d'interaction avec Discord (ban, unban, etc.)
pub struct DiscordApiService {
    token: String,
    client: reqwest::Client,
}

/// Plafond de duree d'un appel Discord. Le defaut de reqwest est ILLIMITE : un
/// incident cote Discord bloquait une requete du panel indefiniment, et avec
/// elle un worker tokio. 30 s laisse largement passer les appels lents
/// (listes de membres) tout en bornant le pire cas.
const HTTP_TIMEOUT_SECS: u64 = 30;

/// Nombre de re-essais apres un 429 (limite de rythme Discord).
const RATE_LIMIT_RETRIES: u32 = 3;
/// Attente maximale honoree entre deux essais. Au-dela, on rend la main a
/// l'appelant plutot que de tenir la requete HTTP du panel ouverte.
const RATE_LIMIT_MAX_WAIT_SECS: f64 = 5.0;

impl DiscordApiService {
    pub fn new(token: String) -> Self {
        Self {
            token,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
                .build()
                // Un builder qui echoue signifierait un environnement TLS
                // casse ; on retombe sur le client par defaut plutot que de
                // faire paniquer le demarrage de l'API.
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.token.is_empty()
    }

    fn ensure_configured(&self) -> Result<(), DomainError> {
        if self.token.is_empty() {
            return Err(DomainError::Internal(
                "SENTINEL_DISCORD_TOKEN non configure".into(),
            ));
        }
        Ok(())
    }
}

/// Refuse tout identifiant qui n'est pas un snowflake Discord.
///
/// Ces identifiants viennent de la configuration par serveur, donc de la base,
/// donc indirectement de saisies utilisateur. Ils sont interpoles dans une URL
/// appelee avec le token du bot : un `../` ou un `%2F` permettrait d'atteindre
/// un tout autre endpoint de l'API Discord. La fenetre 17-20 chiffres ASCII
/// ecarte aussi les uuid et les petits entiers.
///
/// Le controle vit ici et non chez l'appelant : c'est l'adaptateur qui
/// construit l'URL, c'est donc a lui de garantir qu'elle est sure. Chaque
/// appelant qui le refaisait pouvait l'oublier.
fn ensure_snowflake(id: &str) -> Result<(), DomainError> {
    let valide = (17..=20).contains(&id.len()) && id.chars().all(|c| c.is_ascii_digit());
    if valide {
        Ok(())
    } else {
        Err(DomainError::ValidationError(format!(
            "identifiant Discord invalide : {id:?}"
        )))
    }
}

/// Delai d'attente demande par Discord apres un 429, en secondes.
///
/// L'en-tete `Retry-After` fait foi. Discord le renvoie parfois en
/// fractions de seconde (« 0.75 »), d'ou le parse en flottant.
fn retry_after_secs(resp: &reqwest::Response) -> Option<f64> {
    resp.headers()
        .get("retry-after")?
        .to_str()
        .ok()?
        .parse::<f64>()
        .ok()
        .filter(|v| *v >= 0.0)
}

/// Envoie une requete en respectant la limite de rythme Discord.
///
/// Un 429 n'est pas une erreur de l'appelant : c'est Discord qui demande
/// d'attendre. Sur la creation d'un plan de salons, un seul 429 non gere
/// faisait echouer tous les salons suivants — l'attente rend l'operation
/// simplement plus lente au lieu de la casser a moitie.
async fn send_with_rate_limit<F>(build: F) -> Result<reqwest::Response, DomainError>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let mut attempts = 0;
    loop {
        let resp = build()
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if resp.status().as_u16() != 429 || attempts >= RATE_LIMIT_RETRIES {
            return Ok(resp);
        }
        let wait = retry_after_secs(&resp)
            .unwrap_or(1.0)
            .min(RATE_LIMIT_MAX_WAIT_SECS);
        tracing::debug!(wait, attempts, "Discord 429 : attente avant re-essai");
        tokio::time::sleep(std::time::Duration::from_secs_f64(wait)).await;
        attempts += 1;
    }
}

/// Parse une reponse `GET /guilds/{id}/channels` Discord et convertit chaque
/// salon en `DiscordChannel`. `kind_for_type` retourne `Some(label)` pour les
/// types de salons a inclure et `None` pour les autres (categorie, thread...).
async fn parse_channels(
    resp: reqwest::Response,
    kind_for_type: impl Fn(u64) -> Option<&'static str>,
) -> Result<Vec<DiscordChannel>, DomainError> {
    let raw: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| DomainError::Internal(format!("Discord list channels parse: {e}")))?;
    let mut channels: Vec<DiscordChannel> = raw
        .into_iter()
        .filter_map(|c| {
            let ty = c.get("type").and_then(|v| v.as_u64()).unwrap_or(999);
            let kind = kind_for_type(ty)?.to_string();
            let id = c.get("id").and_then(|v| v.as_str())?.to_string();
            let name = c.get("name").and_then(|v| v.as_str())?.to_string();
            let position = c.get("position").and_then(|v| v.as_i64()).unwrap_or(0);
            Some(DiscordChannel {
                id,
                name,
                position,
                kind,
            })
        })
        .collect();
    channels.sort_by_key(|c| c.position);
    Ok(channels)
}

#[async_trait]
impl DiscordApi for DiscordApiService {
    /// Liste les salons texte d'un serveur Discord (id + name).
    /// Phase 9 Part E : utilise par la page web de config des railleries
    /// pour afficher un dropdown au lieu d'un input ID.
    async fn list_text_channels(&self, guild_id: &str) -> Result<Vec<DiscordChannel>, DomainError> {
        self.ensure_configured()?;

        let url = format!("https://discord.com/api/v10/guilds/{}/channels", guild_id);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord list channels failed ({status}): {body}"
            )));
        }

        parse_channels(resp, |ty| match ty {
            0 => Some("text"),
            5 => Some("announcement"),
            _ => None,
        })
        .await
    }

    async fn list_all_channels(&self, guild_id: &str) -> Result<Vec<DiscordChannel>, DomainError> {
        self.ensure_configured()?;

        let url = format!("https://discord.com/api/v10/guilds/{}/channels", guild_id);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord list channels failed ({status}): {body}"
            )));
        }

        parse_channels(resp, |ty| match ty {
            0 => Some("text"),
            5 => Some("announcement"),
            2 => Some("voice"),
            4 => Some("category"),
            13 => Some("stage"),
            // Les forums sont creables depuis le constructeur de salons : les
            // omettre ici les rendait invisibles apres creation, donc
            // impossibles a supprimer depuis le panel et faciles a dupliquer.
            15 => Some("forum"),
            _ => None,
        })
        .await
    }

    /// Cree un salon / une categorie. Renvoie l'ID du salon cree.
    ///
    /// Les overwrites arrivent deja calcules (bits allow/deny) : le choix des
    /// permissions est une regle metier, pas un detail de transport.
    async fn create_channel(
        &self,
        guild_id: &str,
        spec: &NewChannel<'_>,
    ) -> Result<String, DomainError> {
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/guilds/{}/channels", guild_id);

        let mut body = serde_json::json!({
            "name": spec.name,
            "type": spec.kind,
            "nsfw": spec.nsfw,
        });
        if let Some(parent) = spec.parent_id {
            body["parent_id"] = serde_json::Value::String(parent.to_string());
        }
        if let Some(topic) = spec.topic.filter(|t| !t.is_empty()) {
            body["topic"] = serde_json::Value::String(topic.to_string());
        }
        if spec.slowmode > 0 {
            body["rate_limit_per_user"] = serde_json::json!(spec.slowmode);
        }
        if let Some(limit) = spec.user_limit {
            body["user_limit"] = serde_json::json!(limit);
        }
        if !spec.overwrites.is_empty() {
            // Les bitfields Discord se transportent en CHAINES : ils depassent
            // la precision entiere de JSON.
            body["permission_overwrites"] = serde_json::Value::Array(
                spec.overwrites
                    .iter()
                    .map(|ow| {
                        serde_json::json!({
                            "id": ow.role_id,
                            "type": 0, // 0 = role
                            "allow": ow.allow.to_string(),
                            "deny": ow.deny.to_string(),
                        })
                    })
                    .collect(),
            );
        }

        let resp = send_with_rate_limit(|| {
            self.client
                .post(&url)
                .header("Authorization", format!("Bot {}", self.token))
                .json(&body)
        })
        .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            // Ces messages atterrissent tels quels dans le compte rendu du
            // panel, en face du salon concerne : ils doivent dire quoi faire,
            // pas recracher la reponse de Discord.
            return Err(match status.as_u16() {
                403 => DomainError::Forbidden(
                    "Le bot n'a pas la permission « Gerer les salons » sur ce serveur, ou ne peut \
                     pas accorder une permission qu'il ne possede pas lui-meme."
                        .into(),
                ),
                429 => DomainError::RateLimited(
                    "Discord limite le rythme de creation. Les salons restants n'ont pas ete \
                     crees : relancez le plan dans quelques instants."
                        .into(),
                ),
                400 if body.contains("Maximum number of channels") => DomainError::ValidationError(
                    "Le serveur a atteint la limite Discord de 500 salons.".into(),
                ),
                400 => DomainError::ValidationError(format!("Salon refuse par Discord : {body}")),
                404 => DomainError::NotFound(
                    "Serveur Discord introuvable (le bot y est-il present ?)".into(),
                ),
                _ => DomainError::Internal(format!("Discord create_channel ({status}): {body}")),
            });
        }

        let created: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("Parse error: {e}")))?;
        created
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| DomainError::Internal("Discord create_channel: reponse sans id".into()))
    }

    /// Supprime un salon / une categorie. Supprimer une categorie ne supprime
    /// pas ses salons (Discord les remonte a la racine) : c'est le
    /// comportement natif, on ne le contredit pas ici.
    async fn delete_channel(&self, channel_id: &str) -> Result<(), DomainError> {
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/channels/{}", channel_id);

        let resp = send_with_rate_limit(|| {
            self.client
                .delete(&url)
                .header("Authorization", format!("Bot {}", self.token))
        })
        .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord delete_channel failed ({status}): {body}"
            )));
        }
        Ok(())
    }

    async fn list_emojis(&self, guild_id: &str) -> Result<Vec<DiscordEmoji>, DomainError> {
        ensure_snowflake(guild_id)?;
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/guilds/{guild_id}/emojis");
        
        let res = self.client.get(&url).header("Authorization", format!("Bot {}", self.token)).send().await;
        let res = match res {
            Ok(r) => r,
            Err(e) => return Err(DomainError::Internal(format!("HTTP error list_emojis: {e}"))),
        };
        
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!("Discord API list_emojis {status}: {body}")));
        }
        
        let emojis: Vec<DiscordEmoji> = res.json().await.map_err(|e| DomainError::Internal(format!("Parse error list_emojis: {e}")))?;
        Ok(emojis)
    }

    /// Upload un emoji custom sur un serveur Discord.
    /// `image_bytes` : PNG/JPG/GIF < 256 KB. Retourne (emoji_id, emoji_name).
    /// Le bot doit avoir la permission MANAGE_GUILD_EXPRESSIONS sur la guild.
    async fn upload_emoji(
        &self,
        guild_id: &str,
        name: &str,
        image_bytes: &[u8],
        mime: &str,
    ) -> Result<(String, String, bool), DomainError> {
        self.ensure_configured()?;

        if image_bytes.len() > 256 * 1024 {
            return Err(DomainError::ValidationError(
                "L'image depasse 256 KB apres encodage.".into(),
            ));
        }

        let b64 = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(image_bytes)
        };
        let data_uri = format!("data:{};base64,{}", mime, b64);

        let url = format!("https://discord.com/api/v10/guilds/{}/emojis", guild_id);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({
                "name": name,
                "image": data_uri,
            }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                403 => DomainError::Forbidden(format!(
                    "Le bot n'a pas la permission de gerer les emojis sur ce serveur. {body}"
                )),
                429 => DomainError::RateLimited(format!(
                    "Trop de requetes vers Discord, reessayez dans quelques instants. {body}"
                )),
                400 if body.contains("Maximum number of emojis") => DomainError::ValidationError(
                    "Le serveur d'hebergement est plein (quota d'emojis atteint).".into(),
                ),
                400 => DomainError::ValidationError(format!("Image ou nom invalide : {body}")),
                404 => DomainError::NotFound(
                    "Serveur Discord introuvable (le bot y est-il present ?)".into(),
                ),
                _ => DomainError::Internal(format!("Discord upload emoji ({status}): {body}")),
            });
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord upload emoji parse: {e}")))?;
        let id = body
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::Internal("Discord n'a pas renvoye l'id de l'emoji".into()))?
            .to_string();
        let returned_name = body
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(name)
            .to_string();
        let animated = body
            .get("animated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Ok((id, returned_name, animated))
    }

    /// Bannir un utilisateur d'un serveur Discord.
    async fn ban_user(
        &self,
        guild_id: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;

        let url = format!(
            "https://discord.com/api/v10/guilds/{}/bans/{}",
            guild_id, user_id
        );

        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({
                "delete_message_seconds": 86400,
                "reason": reason,
            }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord ban failed ({status}): {body}"
            )));
        }

        Ok(())
    }

    /// Recuperer la liste des membres d'un serveur Discord (id + username).
    async fn list_members(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<DiscordMember>, DomainError> {
        self.ensure_configured()?;

        let mut all_members = Vec::new();
        let mut after: Option<String> = None;
        let page_size = std::cmp::min(limit, 1000);

        loop {
            let mut url = format!(
                "https://discord.com/api/v10/guilds/{}/members?limit={}",
                guild_id, page_size
            );
            if let Some(ref after_id) = after {
                url.push_str(&format!("&after={}", after_id));
            }

            let resp = self
                .client
                .get(&url)
                .header("Authorization", format!("Bot {}", self.token))
                .send()
                .await
                .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(DomainError::Internal(format!(
                    "Discord list members failed ({status}): {body}"
                )));
            }

            let members: Vec<serde_json::Value> = resp
                .json()
                .await
                .map_err(|e| DomainError::Internal(format!("Discord parse error: {e}")))?;

            if members.is_empty() {
                break;
            }

            for m in &members {
                let user = match m.get("user") {
                    Some(u) => u,
                    None => continue,
                };

                let id = user
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let username = user
                    .get("username")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let display_name = m
                    .get("nick")
                    .and_then(|v| v.as_str())
                    .or_else(|| user.get("global_name").and_then(|v| v.as_str()))
                    .map(|s| s.to_string());

                let avatar_hash = user.get("avatar").and_then(|v| v.as_str());
                let avatar_url = discord_avatar_url(&id, avatar_hash);

                if !id.is_empty() {
                    all_members.push(DiscordMember {
                        id,
                        username,
                        display_name,
                        avatar_url,
                    });
                }
            }

            if all_members.len() >= limit as usize || members.len() < page_size as usize {
                break;
            }

            after = all_members.last().map(|m| m.id.clone());
        }

        Ok(all_members)
    }

    /// Envoyer un message prive a un utilisateur Discord.
    async fn send_dm(&self, user_id: &str, content: &str) -> Result<(), DomainError> {
        self.ensure_configured()?;

        // 1. Creer un canal DM
        let dm_resp = self
            .client
            .post("https://discord.com/api/v10/users/@me/channels")
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "recipient_id": user_id }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord DM channel error: {e}")))?;

        if !dm_resp.status().is_success() {
            let body = dm_resp.text().await.unwrap_or_default();
            tracing::warn!("Impossible d'ouvrir un DM avec {user_id}: {body}");
            return Ok(()); // Ne pas faire echouer la suppression si le DM echoue
        }

        let channel: serde_json::Value = dm_resp
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord DM parse error: {e}")))?;
        let channel_id = channel["id"].as_str().unwrap_or_default();

        // 2. Envoyer le message
        let msg_resp = self
            .client
            .post(format!(
                "https://discord.com/api/v10/channels/{channel_id}/messages"
            ))
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "content": content }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord send DM error: {e}")))?;

        if !msg_resp.status().is_success() {
            let body = msg_resp.text().await.unwrap_or_default();
            tracing::warn!("Echec envoi DM a {user_id}: {body}");
        }

        Ok(())
    }

    /// Poste un embed dans un salon.
    async fn send_channel_embed(
        &self,
        channel_id: &str,
        embed: serde_json::Value,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;
        ensure_snowflake(channel_id)?;

        let resp = self
            .client
            .post(format!(
                "https://discord.com/api/v10/channels/{channel_id}/messages"
            ))
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "embeds": [embed] }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord send embed error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord a refuse l'embed (HTTP {status}) : {body}"
            )));
        }
        Ok(())
    }

    // ── Gestion des roles ──

    /// Liste les roles du serveur EN DIRECT (source : Discord, pas la table
    /// `discord_roles` synchronisee). Trie du plus haut au plus bas, comme
    /// Discord les affiche.
    async fn list_roles(&self, guild_id: &str) -> Result<Vec<DiscordRoleInfo>, DomainError> {
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/guilds/{}/roles", guild_id);

        let resp = send_with_rate_limit(|| {
            self.client
                .get(&url)
                .header("Authorization", format!("Bot {}", self.token))
        })
        .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord list_roles failed ({status}): {body}"
            )));
        }

        let raw: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord list_roles parse: {e}")))?;
        let mut roles: Vec<DiscordRoleInfo> = raw
            .into_iter()
            .filter_map(|r| {
                Some(DiscordRoleInfo {
                    id: r.get("id")?.as_str()?.to_string(),
                    name: r.get("name")?.as_str()?.to_string(),
                    color: r.get("color").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    position: r.get("position").and_then(|v| v.as_i64()).unwrap_or(0),
                    managed: r.get("managed").and_then(|v| v.as_bool()).unwrap_or(false),
                })
            })
            .collect();
        roles.sort_by(|a, b| b.position.cmp(&a.position));
        Ok(roles)
    }

    /// Creer un role Discord.
    async fn create_role(
        &self,
        guild_id: &str,
        name: &str,
        color: u32,
        permissions: Option<&str>,
    ) -> Result<serde_json::Value, DomainError> {
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/guilds/{}/roles", guild_id);

        let mut body = serde_json::json!({
            "name": name,
            "color": color,
            "mentionable": false,
        });
        if let Some(perms) = permissions {
            body["permissions"] = serde_json::Value::String(perms.to_string());
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord create_role failed: {body}"
            )));
        }

        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| DomainError::Internal(format!("Parse error: {e}")))
    }

    /// Modifier un role Discord.
    async fn edit_role(
        &self,
        guild_id: &str,
        role_id: &str,
        name: Option<&str>,
        color: Option<u32>,
        permissions: Option<&str>,
        mentionable: Option<bool>,
        hoist: Option<bool>,
    ) -> Result<serde_json::Value, DomainError> {
        self.ensure_configured()?;
        let url = format!(
            "https://discord.com/api/v10/guilds/{}/roles/{}",
            guild_id, role_id
        );

        let mut body = serde_json::json!({});
        if let Some(n) = name {
            body["name"] = serde_json::Value::String(n.to_string());
        }
        if let Some(c) = color {
            body["color"] = serde_json::json!(c);
        }
        if let Some(p) = permissions {
            body["permissions"] = serde_json::Value::String(p.to_string());
        }
        if let Some(m) = mentionable {
            body["mentionable"] = serde_json::json!(m);
        }
        if let Some(h) = hoist {
            body["hoist"] = serde_json::json!(h);
        }

        let resp = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord edit_role failed: {body}"
            )));
        }

        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| DomainError::Internal(format!("Parse error: {e}")))
    }

    /// Supprimer un role Discord.
    async fn delete_role(&self, guild_id: &str, role_id: &str) -> Result<(), DomainError> {
        self.ensure_configured()?;
        let url = format!(
            "https://discord.com/api/v10/guilds/{}/roles/{}",
            guild_id, role_id
        );

        let resp = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord delete_role failed: {body}"
            )));
        }

        Ok(())
    }

    /// Debannir un utilisateur d'un serveur Discord.
    async fn unban_user(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.ensure_configured()?;

        let url = format!(
            "https://discord.com/api/v10/guilds/{}/bans/{}",
            guild_id, user_id
        );

        let resp = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        let status = resp.status();
        if !status.is_success() && status.as_u16() != 404 {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord unban failed ({status}): {body}"
            )));
        }

        Ok(())
    }

    /// Retire le timeout (mute) d'un membre en mettant
    /// `communication_disabled_until` a null via PATCH /guilds/{guild_id}/members/{user_id}.
    ///
    /// Si le membre n'a pas de timeout actif, Discord accepte quand meme la
    /// requete (no-op). Un 404 (user pas dans la guild) est tolere comme un
    /// succes logique — on veut juste qu'il n'y ait plus de timeout actif.
    async fn remove_timeout(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.ensure_configured()?;

        let url = format!(
            "https://discord.com/api/v10/guilds/{}/members/{}",
            guild_id, user_id
        );

        let resp = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "communication_disabled_until": null }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        let status = resp.status();
        if !status.is_success() && status.as_u16() != 404 {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord remove_timeout failed ({status}): {body}"
            )));
        }

        Ok(())
    }

    /// Applique un timeout (mute Discord) sur un membre pour une duree donnee
    /// en secondes, via PATCH /guilds/{guild_id}/members/{user_id} avec
    /// `communication_disabled_until = now + duration`.
    ///
    /// Discord limite le timeout a 28 jours max — on clamp automatiquement.
    async fn apply_timeout(
        &self,
        guild_id: &str,
        user_id: &str,
        duration_seconds: u64,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;

        // Discord max : 28 jours (2 419 200 secondes).
        const MAX_TIMEOUT_SECS: u64 = 28 * 24 * 3600;
        let dur = duration_seconds.min(MAX_TIMEOUT_SECS);
        let until = chrono::Utc::now() + chrono::Duration::seconds(dur as i64);
        let until_str = until.to_rfc3339();

        let url = format!(
            "https://discord.com/api/v10/guilds/{}/members/{}",
            guild_id, user_id
        );

        let resp = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "communication_disabled_until": until_str }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord apply_timeout failed ({status}): {body}"
            )));
        }

        Ok(())
    }

    /// Phase 2 B — Recupere la liste des guilds auxquelles un user appartient.
    /// Utilise le `access_token` OAuth2 (Bearer) du user, PAS le bot token.
    /// Endpoint Discord : `GET /users/@me/guilds` (scope `identify` ou `guilds`).
    async fn get_user_guilds(&self, access_token: &str) -> Result<Vec<UserGuild>, DomainError> {
        let url = "https://discord.com/api/v10/users/@me/guilds";
        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord guilds fetch failed: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord get_user_guilds non-success ({status}): {body}"
            )));
        }

        resp.json::<Vec<UserGuild>>()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord guilds parse: {e}")))
    }

    /// Phase 7 B — Recupere l'identite du user associe a un `access_token`.
    /// Endpoint Discord : `GET /users/@me` (scope `identify`).
    async fn get_user_me(&self, access_token: &str) -> Result<DiscordUser, DomainError> {
        let url = "https://discord.com/api/v10/users/@me";
        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord /users/@me fetch: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord get_user_me non-success ({status}): {body}"
            )));
        }

        resp.json::<DiscordUser>()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord /users/@me parse: {e}")))
    }
}

/// Construit l'URL d'avatar Discord (CDN) pour un user.
/// Retourne `None` si le user n'a pas d'avatar custom (hash absent).
pub(super) fn discord_avatar_url(user_id: &str, avatar_hash: Option<&str>) -> Option<String> {
    avatar_hash.map(|h| {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png?size=64",
            user_id, h
        )
    })
}

#[cfg(test)]
#[path = "tests/discord_api.rs"]
mod tests;
