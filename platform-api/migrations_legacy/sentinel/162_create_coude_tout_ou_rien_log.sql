-- Migration 162 : Memorial des clodos (cf. COUPE_AMELIORATIONS 6.1).
--
-- Logue chaque tentative de /tout-ou-rien : mise + outcome (won/lost) +
-- delta. Le "Memorial des clodos" est un leaderboard public des plus
-- grosses pertes — humiliation collective.

CREATE TABLE IF NOT EXISTS coude_tout_ou_rien_log (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    VARCHAR(20) NOT NULL,
    user_id     VARCHAR(20) NOT NULL,
    username    VARCHAR(100) NOT NULL,
    mise        BIGINT NOT NULL,
    outcome     VARCHAR(8) NOT NULL,
    delta       BIGINT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (outcome IN ('won', 'lost'))
);

-- Lookup principal pour le Memorial : top pertes (delta le plus negatif).
CREATE INDEX IF NOT EXISTS idx_coude_tout_ou_rien_log_memorial
    ON coude_tout_ou_rien_log (guild_id, delta ASC)
    WHERE outcome = 'lost';

-- Lookup historique par user pour /profil (anciens TOR).
CREATE INDEX IF NOT EXISTS idx_coude_tout_ou_rien_log_user
    ON coude_tout_ou_rien_log (guild_id, user_id, created_at DESC);
