use super::*;

impl ApiClient {
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

    /// GET /api/games/{guild_id}/panels/{message_id}.
    pub async fn find_panel_by_message(
        &self,
        guild_id: &str,
        message_id: &str,
    ) -> Result<Option<GamePanel>, String> {
        let url = format!(
            "{}/api/games/{}/panels/{}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(message_id)
        );
        self.send(self.http.get(&url)).await
    }

    // ── Consolidation base <-> Discord ──

    /// PUT /api/games/{guild_id}/sync/inventory — depose la photographie de la
    /// guilde. Le bot est le seul a voir Discord : sans ce depot, l'API ne peut
    /// constater aucune divergence.
    pub async fn put_sync_inventory(
        &self,
        guild_id: &str,
        inventory: &serde_json::Value,
    ) -> Result<(), String> {
        let url = format!(
            "{}/api/games/{}/sync/inventory",
            self.base_url,
            encode_segment(guild_id)
        );
        self.send_no_content(self.http.put(&url).json(inventory))
            .await
    }

    /// DELETE /api/games/{guild_id}/sync/roles/{role_id} — signale un role
    /// disparu de Discord, pour que la liaison cesse tout de suite d'etre
    /// utilisee. Le jeu n'est pas supprime : cela reste une decision humaine.
    pub async fn report_vanished_role(&self, guild_id: &str, role_id: &str) -> Result<(), String> {
        let url = format!(
            "{}/api/games/{}/sync/roles/{}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(role_id)
        );
        self.send_no_content(self.http.delete(&url)).await
    }
}
