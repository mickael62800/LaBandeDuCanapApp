-- Coup de Coude v2 : HP system, multi-rounds, seasons

-- HP persistants sur les joueurs
ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS hp_current INTEGER NOT NULL DEFAULT 100;
ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS hp_max INTEGER NOT NULL DEFAULT 100;
ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS hp_last_regen TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Changement de classe
ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS class_changed_at TIMESTAMPTZ;

-- Saisons
ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS season INTEGER NOT NULL DEFAULT 1;

-- Repos (full heal cooldown)
ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS repos_last_used TIMESTAMPTZ;

-- Combat v2 : rounds data + surenchere
ALTER TABLE coude_combats ADD COLUMN IF NOT EXISTS rounds_data JSONB;
ALTER TABLE coude_combats ADD COLUMN IF NOT EXISTS channel_id_temp TEXT;
ALTER TABLE coude_combats ADD COLUMN IF NOT EXISTS final_mise BIGINT;
