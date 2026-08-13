-- Sabot partage : le deck est dans la table, pas dans chaque partie.
-- 6 decks melanges = 312 cartes (standard casino).
ALTER TABLE blackjack_tables
    ADD COLUMN IF NOT EXISTS deck JSONB NOT NULL DEFAULT '[]',
    ADD COLUMN IF NOT EXISTS dealer_hand JSONB NOT NULL DEFAULT '[]',
    ADD COLUMN IF NOT EXISTS dealer_score INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS round_status TEXT NOT NULL DEFAULT 'waiting',
    ADD COLUMN IF NOT EXISTS current_player_index INTEGER NOT NULL DEFAULT 0;

-- Config : joueurs max par table (dans bot_definitions)
UPDATE bot_definitions SET config_schema = config_schema::jsonb || '[
    {"key": "max_players_per_table", "label": "Joueurs max par table (defaut 7)", "type": "number", "required": false, "default": "7"}
]'::jsonb WHERE bot_name = 'blackjack-bot';
