-- Phase 9 Part D — Railleries automatiques sur series (streaks).
--
-- Le jeu detecte quand un joueur enchaine victoires, defaites ou vols
-- subis. A partir de seuils configurables (3/5/10 par defaut), un
-- message moqueur est poste dans un salon dedie et le pseudo Discord
-- du joueur est renomme avec un suffixe progressif.
--
-- Les streaks vivent sur `coude_players` parce que c'est le meme
-- cycle de vie que les autres stats joueurs. La config (channel,
-- opt-outs) vit dans des tables dediees pour ne pas alourdir la row.

-- ── Colonnes de streak sur coude_players ──
ALTER TABLE coude_players
    ADD COLUMN IF NOT EXISTS current_win_streak INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS current_loss_streak INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS current_steal_victim_streak INT NOT NULL DEFAULT 0;

-- ── Config par guild : salon dedie aux railleries ──
CREATE TABLE IF NOT EXISTS coude_taunts_config (
    guild_id TEXT PRIMARY KEY,
    channel_id TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Opt-outs individuels ──
--
-- Un joueur qui /no-taunts on insere ici. /no-taunts off supprime la
-- ligne. La presence de la ligne = opt-out actif.
CREATE TABLE IF NOT EXISTS coude_taunts_opt_outs (
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_coude_taunts_opt_outs_user
  ON coude_taunts_opt_outs (guild_id, user_id);
