-- ============================================
-- Blackjack — jeu de cartes
-- ============================================

CREATE TABLE IF NOT EXISTS blackjack_games (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    username        TEXT NOT NULL DEFAULT '',
    bet             BIGINT NOT NULL,
    player_hand     JSONB NOT NULL DEFAULT '[]',    -- [{"rank":"As","suit":"heart"}, ...]
    dealer_hand     JSONB NOT NULL DEFAULT '[]',
    deck            JSONB NOT NULL DEFAULT '[]',    -- cartes restantes
    status          TEXT NOT NULL DEFAULT 'playing', -- playing, player_bust, dealer_bust, player_win, dealer_win, push, player_blackjack
    player_score    INT NOT NULL DEFAULT 0,
    dealer_score    INT NOT NULL DEFAULT 0,
    doubled         BOOLEAN NOT NULL DEFAULT FALSE,
    payout          BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_blackjack_active ON blackjack_games(guild_id, user_id, status) WHERE status = 'playing';

-- Seed blackjack-bot definition
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES ('blackjack-bot', 'Blackjack', 'Bot de jeu Blackjack — cartes et paris', '{
  "type": "object",
  "properties": {
    "enabled": {"type": "boolean", "default": true, "description": "Activer le bot Blackjack"},
    "min_bet": {"type": "integer", "default": 10, "description": "Mise minimale"},
    "max_bet": {"type": "integer", "default": 1000, "description": "Mise maximale (0 = illimite)"},
    "starting_coins": {"type": "integer", "default": 200, "description": "Coins de depart pour les nouveaux joueurs"},
    "blackjack_payout": {"type": "number", "default": 1.5, "description": "Multiplicateur pour un blackjack (defaut x1.5)"}
  }
}')
ON CONFLICT (bot_name) DO NOTHING;
