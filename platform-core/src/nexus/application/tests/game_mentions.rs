use super::*;
use crate::nexus::ports::outbound::casino::game_repository::{Game, GamePanel};
use async_trait::async_trait;

struct MockGameRepo {
    games: Vec<Game>,
    should_fail: bool,
}

#[async_trait]
impl GameRepository for MockGameRepo {
    async fn list(&self, _guild_id: &str) -> Result<Vec<Game>, DomainError> {
        if self.should_fail {
            return Err(DomainError::Infrastructure("DB error".into()));
        }
        Ok(self.games.clone())
    }
    async fn list_by_category(
        &self,
        _guild_id: &str,
        _cat: Option<&str>,
    ) -> Result<Vec<Game>, DomainError> {
        Ok(self.games.clone())
    }
    async fn find_by_name(
        &self,
        _guild_id: &str,
        _name: &str,
    ) -> Result<Option<Game>, DomainError> {
        Ok(None)
    }
    async fn create(
        &self,
        _guild_id: &str,
        _game_name: &str,
        _created_by: &str,
        _emoji: Option<&str>,
        _category: Option<&str>,
        _role_id: Option<&str>,
    ) -> Result<Game, DomainError> {
        Err(DomainError::NotImplemented("test".into()))
    }
    async fn update(
        &self,
        _guild_id: &str,
        _game_id: &str,
        _game_name: Option<&str>,
        _emoji: Option<Option<&str>>,
        _category: Option<Option<&str>>,
    ) -> Result<Option<Game>, DomainError> {
        Err(DomainError::NotImplemented("test".into()))
    }
    async fn delete(&self, _guild_id: &str, _game_id: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn set_role_id(
        &self,
        _guild_id: &str,
        _game_id: &str,
        _role_id: Option<&str>,
    ) -> Result<Option<Game>, DomainError> {
        Ok(None)
    }
    async fn list_panels(&self, _guild_id: &str) -> Result<Vec<GamePanel>, DomainError> {
        Ok(vec![])
    }
    async fn find_panel_by_message(
        &self,
        _guild_id: &str,
        _msg_id: &str,
    ) -> Result<Option<GamePanel>, DomainError> {
        Ok(None)
    }
    async fn save_panel(
        &self,
        _guild_id: &str,
        _channel_id: &str,
        _message_id: &str,
        _category: Option<&str>,
    ) -> Result<GamePanel, DomainError> {
        Err(DomainError::NotImplemented("test".into()))
    }
    async fn delete_panel(&self, _guild_id: &str, _message_id: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
}

fn sample_game(id: &str, name: &str) -> Game {
    use crate::nexus::domain::entities::system::discord_ids::GuildId;
    Game {
        id: id.into(),
        guild_id: GuildId("guild1".into()),
        game_name: name.into(),
        created_by: "user1".into(),
        created_at: "2026-01-01".into(),
        role_id: Some(format!("role_{id}")),
        emoji: None,
        category: None,
    }
}

#[tokio::test]
async fn test_detect_mentions_exact_and_case_insensitive() {
    let repo = Arc::new(MockGameRepo {
        games: vec![
            sample_game("1", "Minecraft"),
            sample_game("2", "League of Legends"),
            sample_game("3", "Ark"),
            sample_game("4", "C++"),
            sample_game("5", "C#"),
        ],
        should_fail: false,
    });

    let uc = DetectGameMentionsUseCase::new(repo);

    // Case insensitive match
    let res = uc
        .execute("guild1", "Qui est chaud pour du MINECRAFT ce soir ?")
        .await
        .unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].game_name, "Minecraft");

    // Substring boundary check: "remarkable" should not match "Ark"
    let res2 = uc
        .execute("guild1", "C'est remarquable comme jeu !")
        .await
        .unwrap();
    assert_eq!(res2.len(), 0);

    // Special characters match: "C++" and "C#"
    let res3 = uc
        .execute("guild1", "Je developpe un jeu en C++ et C# !")
        .await
        .unwrap();
    assert_eq!(res3.len(), 2);
    assert_eq!(res3[0].game_name, "C++");
    assert_eq!(res3[1].game_name, "C#");

    // Multiple match
    let res4 = uc
        .execute("guild1", "Un minecraft ou un League of Legends ?")
        .await
        .unwrap();
    assert_eq!(res4.len(), 2);
}

#[tokio::test]
async fn test_detect_mentions_error() {
    let repo = Arc::new(MockGameRepo {
        games: vec![],
        should_fail: true,
    });
    let uc = DetectGameMentionsUseCase::new(repo);
    let res = uc.execute("guild1", "Minecraft").await;
    assert!(res.is_err());
}
