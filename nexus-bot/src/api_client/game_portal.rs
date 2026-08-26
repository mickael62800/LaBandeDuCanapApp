use super::*;

impl ApiClient {
    /// GET /api/games/{guild_id}/servers — utilise au demarrage du bot pour
    /// rattraper les evenements de creation de salons manques.
    pub async fn list_game_servers(&self, guild_id: &str) -> Result<Vec<GameServer>, String> {
        let url = format!(
            "{}/api/games/{}/servers",
            self.base_url,
            encode_segment(guild_id)
        );
        self.send(self.http.get(&url)).await
    }

    /// GET /api/games/servers/{server_id}.
    pub async fn get_game_server(&self, server_id: &str) -> Result<ServerDetailResponse, String> {
        let url = format!(
            "{}/api/games/servers/{}",
            self.base_url,
            encode_segment(server_id)
        );
        self.send(self.http.get(&url)).await
    }

    /// Le serveur existe-t-il encore ?
    ///
    /// DISTINGUE L'ABSENCE DE LA PANNE, et c'est tout l'interet de la methode.
    /// `get_game_server` aplatit les deux en `Err(String)` : un 404 et une API
    /// injoignable y sont indiscernables. Utilise pour decider d'une
    /// suppression de salon Discord, cet amalgame effacerait les salons de
    /// TOUTES les sessions vivantes des que l'API tousse.
    ///
    /// Seul un 404 franc repond `Ok(false)`. Tout le reste remonte en `Err` :
    /// l'appelant s'abstient alors, ce qui est la bonne facon d'echouer quand
    /// l'action est irreversible.
    pub async fn game_server_existe(&self, server_id: &str) -> Result<bool, String> {
        let url = format!(
            "{}/api/games/servers/{}",
            self.base_url,
            encode_segment(server_id)
        );
        let mut req = self.http.get(&url);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("nexus-api injoignable: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            return Ok(true);
        }
        if status == reqwest::StatusCode::NOT_FOUND {
            return Ok(false);
        }
        Err(format!("erreur nexus-api ({status})"))
    }

    /// GET /api/games/servers/{id}/announcement — texte redige par Atrium.
    ///
    /// `Ok(None)` veut dire « retente plus tard » : Atrium n'a rien pu ecrire.
    /// `Err` veut dire « ne retente pas », la demande ne passera jamais. Le bot
    /// s'abstient de publier le panneau dans les deux cas, mais seul le premier
    /// merite une reprise.
    pub async fn annonce_de_session(&self, server_id: &str) -> Result<Option<String>, String> {
        #[derive(serde::Deserialize)]
        struct Reponse {
            content: String,
        }

        let url = format!(
            "{}/api/games/servers/{}/announcement",
            self.base_url,
            encode_segment(server_id)
        );
        let mut req = self.http.get(&url);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("nexus-api injoignable: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            return resp
                .json::<Reponse>()
                .await
                .map(|r| Some(r.content))
                .map_err(|e| format!("reponse nexus-api invalide: {e}"));
        }
        if status == reqwest::StatusCode::SERVICE_UNAVAILABLE {
            return Ok(None);
        }
        Err(format!("erreur nexus-api ({status})"))
    }

    /// POST /api/games/servers/{id}/announcement/posted
    pub async fn marquer_annonce_publiee(&self, server_id: &str) -> Result<(), String> {
        let url = format!(
            "{}/api/games/servers/{}/announcement/posted",
            self.base_url,
            encode_segment(server_id)
        );
        self.send_no_content(self.http.post(&url)).await
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

    /// DELETE /api/games/servers/{server_id}/registrations/{user_id}.
    pub async fn unregister_from_server(
        &self,
        server_id: &str,
        user_id: &str,
    ) -> Result<serde_json::Value, String> {
        // Le endpoint exact dans platform-api est :
        // DELETE /api/games/servers/{server_id}/registrations/{user_id}
        let url = format!(
            "{}/api/games/servers/{}/registrations/{}",
            self.base_url,
            encode_segment(server_id),
            encode_segment(user_id)
        );
        self.send(self.http.delete(&url)).await
    }

    /// POST /api/games/servers/{server_id}/reveal-ip/request.
    ///
    /// Flux du bouton : demarre le serveur si besoin et programme la revelation
    /// de l'IP. Renvoie le decompte a annoncer dans le panneau.
    pub async fn request_reveal_ip(
        &self,
        server_id: &str,
        actor_id: &str,
    ) -> Result<RevealRequest, String> {
        let url = format!(
            "{}/api/games/servers/{}/reveal-ip/request",
            self.base_url,
            encode_segment(server_id)
        );
        self.send(self.http.post(&url).query(&[("actor_id", actor_id)]))
            .await
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
