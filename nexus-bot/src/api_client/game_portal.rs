use super::*;

impl ApiClient {
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
}
