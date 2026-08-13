-- Migration 139 — Extensions taunts (blackjack + eco) + tournoi hebdo.
--
-- 3 features livrees ensemble :
--   1. Taunts blackjack : nouvelles colonnes de streak sur coude_players
--      (bj_win_streak / bj_bust_streak) + enum etendu cote Rust.
--   2. Taunts economie : one-shots (faillite, jackpot, don genereux)
--      avec seuils configurables par guild via bot_guild_config.
--   3. Tournoi hebdo : table coude_weekly_tournaments + cles de config.
--
-- Aucun refactor du TauntEvent : on reutilise la meme infra (emit →
-- worker post + rename). Les seuils numeriques eco sont dans le schema
-- de `coude-bot` parce que c'est lie a l'economie Coude.

-- ── Feature 1 : Taunts blackjack — streaks par joueur ──

ALTER TABLE coude_players
    ADD COLUMN IF NOT EXISTS bj_win_streak INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS bj_bust_streak INT NOT NULL DEFAULT 0;

-- ── Feature 3 : Tournoi hebdo ──

CREATE TABLE IF NOT EXISTS coude_weekly_tournaments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    week_start TIMESTAMPTZ NOT NULL,
    week_end TIMESTAMPTZ NOT NULL,
    winner_user_id TEXT,
    winner_username TEXT,
    winner_net_gain BIGINT DEFAULT 0,
    prize_amount BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'ongoing',
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (guild_id, week_start)
);

CREATE INDEX IF NOT EXISTS idx_coude_weekly_tournaments_guild_status
  ON coude_weekly_tournaments (guild_id, status);

CREATE INDEX IF NOT EXISTS idx_coude_weekly_tournaments_week
  ON coude_weekly_tournaments (week_start DESC);

-- ── Features 2 + 3 : cles de config ajoutees au schema coude-bot ──
--
-- On append au JSON existant seulement si les cles ne sont pas deja la
-- (idempotence migration replayable).

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "bankruptcy_taunt_enabled", "label": "Taunt faillite active", "type": "boolean", "required": false, "default": "true", "description": "Raille publiquement un joueur dont le solde passe a 0."},
  {"key": "jackpot_threshold", "label": "Seuil jackpot (coins)", "type": "number", "required": false, "default": "10000", "description": "Gain minimum en une operation pour declencher un taunt jackpot."},
  {"key": "generous_donor_threshold", "label": "Seuil don genereux (coins)", "type": "number", "required": false, "default": "1000", "description": "Montant minimum d un /donner pour declencher un taunt de generosite."},
  {"key": "tournament_enabled", "label": "Tournoi hebdo active", "type": "boolean", "required": false, "default": "true", "description": "Active le classement hebdomadaire des gains nets et la distribution auto du prix du dimanche 23h UTC."},
  {"key": "tournament_prize_pct", "label": "Part du prix hebdo (%)", "type": "number", "required": false, "default": "10", "description": "Pourcentage de la caisse communautaire reverse au gagnant du tournoi."},
  {"key": "tournament_channel_id", "label": "Salon annonce tournoi", "type": "channel", "required": false, "default": "", "description": "Salon ou poster le classement et le resultat hebdo. Si vide, utilise le salon activites."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "tournament_enabled"}]'::jsonb);
