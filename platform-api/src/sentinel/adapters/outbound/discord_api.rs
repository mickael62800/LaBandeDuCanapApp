use async_trait::async_trait;

use platform_core::sentinel::domain::errors::DomainError;
pub use platform_core::sentinel::ports::outbound::discord_api::{
    DiscordApi, DiscordChannel, DiscordEmoji, DiscordMember, DiscordRoleInfo, DiscordUser,
    NewChannel, UserGuild,
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

mod channels;
mod emojis;
mod members;
mod messages;
mod moderation;
mod oauth;
mod roles;

#[async_trait]
impl DiscordApi for DiscordApiService {
    async fn list_text_channels(&self, guild_id: &str) -> Result<Vec<DiscordChannel>, DomainError> {
        self.list_text_channels_impl(guild_id).await
    }

    async fn list_all_channels(&self, guild_id: &str) -> Result<Vec<DiscordChannel>, DomainError> {
        self.list_all_channels_impl(guild_id).await
    }

    async fn create_channel(
        &self,
        guild_id: &str,
        spec: &NewChannel<'_>,
    ) -> Result<String, DomainError> {
        self.create_channel_impl(guild_id, spec).await
    }

    async fn delete_channel(&self, channel_id: &str) -> Result<(), DomainError> {
        self.delete_channel_impl(channel_id).await
    }

    async fn list_emojis(&self, guild_id: &str) -> Result<Vec<DiscordEmoji>, DomainError> {
        self.list_emojis_impl(guild_id).await
    }

    async fn upload_emoji(
        &self,
        guild_id: &str,
        name: &str,
        image_bytes: &[u8],
        mime: &str,
    ) -> Result<(String, String, bool), DomainError> {
        self.upload_emoji_impl(guild_id, name, image_bytes, mime)
            .await
    }

    async fn ban_user(
        &self,
        guild_id: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<(), DomainError> {
        self.ban_user_impl(guild_id, user_id, reason).await
    }

    async fn list_members(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<DiscordMember>, DomainError> {
        self.list_members_impl(guild_id, limit).await
    }

    async fn send_dm(&self, user_id: &str, content: &str) -> Result<(), DomainError> {
        self.send_dm_impl(user_id, content).await
    }

    async fn send_channel_embed(
        &self,
        channel_id: &str,
        embed: serde_json::Value,
    ) -> Result<(), DomainError> {
        self.send_channel_embed_impl(channel_id, embed).await
    }

    async fn list_roles(&self, guild_id: &str) -> Result<Vec<DiscordRoleInfo>, DomainError> {
        self.list_roles_impl(guild_id).await
    }

    async fn create_role(
        &self,
        guild_id: &str,
        name: &str,
        color: u32,
        permissions: Option<&str>,
    ) -> Result<serde_json::Value, DomainError> {
        self.create_role_impl(guild_id, name, color, permissions)
            .await
    }

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
        self.edit_role_impl(
            guild_id,
            role_id,
            name,
            color,
            permissions,
            mentionable,
            hoist,
        )
        .await
    }

    async fn delete_role(&self, guild_id: &str, role_id: &str) -> Result<(), DomainError> {
        self.delete_role_impl(guild_id, role_id).await
    }

    async fn unban_user(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.unban_user_impl(guild_id, user_id).await
    }

    async fn remove_timeout(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.remove_timeout_impl(guild_id, user_id).await
    }

    async fn apply_timeout(
        &self,
        guild_id: &str,
        user_id: &str,
        duration_seconds: u64,
    ) -> Result<(), DomainError> {
        self.apply_timeout_impl(guild_id, user_id, duration_seconds)
            .await
    }

    async fn get_user_guilds(&self, access_token: &str) -> Result<Vec<UserGuild>, DomainError> {
        self.get_user_guilds_impl(access_token).await
    }

    async fn get_user_me(&self, access_token: &str) -> Result<DiscordUser, DomainError> {
        self.get_user_me_impl(access_token).await
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
