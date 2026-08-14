use super::*;

/// Vue bot d'un haut fait : seuls les champs affiches sont deserialises, le
/// reste de la reponse API est ignore.
#[derive(Debug, Deserialize)]
pub struct AchievementProgress {
    pub name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub unlocked_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PlayerLink {
    pub game: String,
    pub game_player_id: String,
}

impl ApiClient {
    /// GET /api/achievements/{guild_id}/members/{user_id}
    pub async fn member_achievements(
        &self,
        guild_id: &str,
        user_id: &str,
        game: Option<&str>,
    ) -> Result<Vec<AchievementProgress>, String> {
        let mut url = format!(
            "{}/api/achievements/{}/members/{}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id)
        );
        if let Some(game) = game {
            url.push_str(&format!("?game={}", encode_segment(game)));
        }
        self.send(self.http.get(&url)).await
    }

    /// GET /api/achievements/{guild_id}/links/{user_id}/{game}
    pub async fn get_player_link(
        &self,
        guild_id: &str,
        user_id: &str,
        game: &str,
    ) -> Result<Option<PlayerLink>, String> {
        let url = self.link_url(guild_id, user_id, game);
        self.send(self.http.get(&url)).await
    }

    /// PUT /api/achievements/{guild_id}/links/{user_id}/{game}
    ///
    /// Le membre declare lui-meme son identite de jeu (SteamID64 pour
    /// Palworld) : c'est ce qui fait office de verification.
    pub async fn link_player(
        &self,
        guild_id: &str,
        user_id: &str,
        game: &str,
        game_player_id: &str,
    ) -> Result<PlayerLink, String> {
        let url = self.link_url(guild_id, user_id, game);
        let body = serde_json::json!({ "game_player_id": game_player_id });
        self.send(self.http.put(&url).json(&body)).await
    }

    /// DELETE /api/achievements/{guild_id}/links/{user_id}/{game}
    pub async fn unlink_player(
        &self,
        guild_id: &str,
        user_id: &str,
        game: &str,
    ) -> Result<(), String> {
        let url = self.link_url(guild_id, user_id, game);
        self.send_no_content(self.http.delete(&url)).await
    }

    fn link_url(&self, guild_id: &str, user_id: &str, game: &str) -> String {
        format!(
            "{}/api/achievements/{}/links/{}/{}",
            self.base_url,
            encode_segment(guild_id),
            encode_segment(user_id),
            encode_segment(game)
        )
    }
}
