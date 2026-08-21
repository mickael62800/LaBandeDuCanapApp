use regex::Regex;
use std::sync::Arc;

use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::outbound::casino::game_repository::{Game, GameRepository};

pub struct DetectGameMentionsUseCase {
    pub game_repo: Arc<dyn GameRepository>,
}

impl DetectGameMentionsUseCase {
    pub fn new(game_repo: Arc<dyn GameRepository>) -> Self {
        Self { game_repo }
    }

    /// Detecte les jeux mentionnes dans un texte.
    /// Retourne la liste des jeux dont le `game_name` apparait dans `content`.
    /// On utilise une recherche avec limites de mots (word boundaries) insensible a la casse.
    pub async fn execute(&self, guild_id: &str, content: &str) -> Result<Vec<Game>, DomainError> {
        let games = self.game_repo.list(guild_id).await?;
        let mut detected_games = Vec::new();

        for game in games {
            // Echapper le nom du jeu au cas ou il contient des caracteres speciaux regex
            let escaped_name = regex::escape(&game.game_name);
            // Construction d'une regex insensible a la casse qui respecte les frontieres de mots/ponctuation
            // (supporte les jeux avec caracteres speciaux comme C++, C#, DotA 2)
            let pattern = format!(r"(?i)(?:^|[\s\p{{P}}]){}(?:$|[\s\p{{P}}])", escaped_name);

            if let Ok(re) = Regex::new(&pattern) {
                if re.is_match(content) {
                    detected_games.push(game);
                }
            }
        }

        Ok(detected_games)
    }
}

#[cfg(test)]
#[path = "tests/game_mentions.rs"]
mod tests;
