-- Automod — regroupement des alertes par utilisateur (anti-flood de cartes).
--
-- Avant : une carte de vote par message signale -> si un user spamme, on
-- pouvait se retrouver avec des dizaines de cartes.
--
-- Apres (si vote_aggregate_enabled) : tant qu'une carte est ouverte (status
-- 'voting') pour un (guild, user), les nouveaux signalements s'y AGREGENT au
-- lieu de creer une nouvelle carte. On accumule la liste des incidents, le
-- nombre, le score cumule, on garde le score max, et on prolonge la deadline.

ALTER TABLE automod_reviews
    ADD COLUMN IF NOT EXISTS incident_count INT NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS cumulative_score DOUBLE PRECISION NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS incidents JSONB NOT NULL DEFAULT '[]'::jsonb;

-- Index pour retrouver vite la carte ouverte d'un utilisateur lors du merge.
CREATE INDEX IF NOT EXISTS idx_automod_reviews_open_user
    ON automod_reviews (guild_id, user_id)
    WHERE status = 'voting';

-- Cle de config (page web automod) : toggle du regroupement.
-- Idempotent : on retire d'abord la cle si presente, puis on la (re)ajoute.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'vote_aggregate_enabled'
        UNION ALL SELECT '{
            "key": "vote_aggregate_enabled",
            "label": "Regrouper les alertes par utilisateur (1 carte/personne)",
            "type": "boolean",
            "required": false,
            "default": "false",
            "description": "Si ON, tant qu''une carte de vote est ouverte pour un membre, les nouveaux signalements s''y ajoutent (liste d''incidents + score cumule + deadline prolongee) au lieu de creer une nouvelle carte. Evite le flood de cartes quand un membre derape en serie."
        }'::jsonb AS elem
    ) sub
)
WHERE bot_name = 'automod-bot';
