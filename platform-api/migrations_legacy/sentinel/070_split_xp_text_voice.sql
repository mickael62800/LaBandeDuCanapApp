-- Dissociation XP texte / vocal avec niveaux separes.

-- Colonnes XP et niveau par source
ALTER TABLE user_levels
    ADD COLUMN xp_text  BIGINT  NOT NULL DEFAULT 0,
    ADD COLUMN xp_voice BIGINT  NOT NULL DEFAULT 0,
    ADD COLUMN level_text  INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN level_voice INTEGER NOT NULL DEFAULT 0;

-- Initialiser xp_text avec l'XP existant (on considere que tout l'ancien XP est du texte)
UPDATE user_levels SET xp_text = xp, level_text = level;

-- Index pour les leaderboards par source
CREATE INDEX IF NOT EXISTS idx_user_levels_xp_text
    ON user_levels (guild_id, xp_text DESC);
CREATE INDEX IF NOT EXISTS idx_user_levels_xp_voice
    ON user_levels (guild_id, xp_voice DESC);

-- Ajouter la source aux recompenses de role (text ou voice)
ALTER TABLE level_rewards
    ADD COLUMN source TEXT NOT NULL DEFAULT 'text';

-- Supprimer l'ancienne contrainte unique et en creer une nouvelle avec source
ALTER TABLE level_rewards
    DROP CONSTRAINT IF EXISTS uq_level_rewards_guild_level;
ALTER TABLE level_rewards
    ADD CONSTRAINT uq_level_rewards_guild_level_source UNIQUE (guild_id, level, source);
