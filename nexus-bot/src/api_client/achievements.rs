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
}
