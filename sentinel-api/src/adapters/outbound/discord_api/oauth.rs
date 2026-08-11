use super::*;

impl DiscordApiService {
    pub(super) async fn get_user_guilds_impl(
        &self,
        access_token: &str,
    ) -> Result<Vec<UserGuild>, DomainError> {
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

    pub(super) async fn get_user_me_impl(
        &self,
        access_token: &str,
    ) -> Result<DiscordUser, DomainError> {
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
