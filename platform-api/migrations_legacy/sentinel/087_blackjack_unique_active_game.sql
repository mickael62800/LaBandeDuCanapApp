-- Empecher les doubles parties de blackjack actives par joueur (race condition)
CREATE UNIQUE INDEX IF NOT EXISTS idx_blackjack_unique_active
    ON blackjack_games (guild_id, user_id)
    WHERE status = 'playing';
