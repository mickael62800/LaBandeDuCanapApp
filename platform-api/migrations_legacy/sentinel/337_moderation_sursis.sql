-- Moderation-bot — « ban en sursis » : au lieu d'un ban Discord direct, on met
-- un role « Sursis » (ne voit que le reglement + son salon d'appel). Le membre a
-- N jours pour contester ; sinon un worker le bannit definitivement.

-- Etat des sursis en cours.
CREATE TABLE IF NOT EXISTS moderation_sursis (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id       TEXT NOT NULL,
    user_id        TEXT NOT NULL,
    username       TEXT NOT NULL DEFAULT '',
    moderator_id   TEXT NOT NULL DEFAULT '',
    moderator_name TEXT NOT NULL DEFAULT '',
    reason         TEXT NOT NULL DEFAULT '',
    saved_roles    JSONB NOT NULL DEFAULT '[]'::jsonb,  -- roles a restaurer si gracie
    channel_id     TEXT,                                -- salon d'appel cree
    status         TEXT NOT NULL DEFAULT 'en_sursis',   -- en_sursis|gracie|banni
    expires_at     TIMESTAMPTZ NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Un seul sursis actif par (guild, user).
CREATE UNIQUE INDEX IF NOT EXISTS uq_moderation_sursis_active
    ON moderation_sursis (guild_id, user_id) WHERE status = 'en_sursis';

-- Scan worker : sursis arrives a echeance.
CREATE INDEX IF NOT EXISTS idx_moderation_sursis_due
    ON moderation_sursis (expires_at) WHERE status = 'en_sursis';

-- Config (parametrable).
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' NOT IN ('sursis_role_id', 'sursis_appeal_days')
        UNION ALL SELECT '{
            "key": "sursis_role_id",
            "label": "Rôle Sursis (ban avec appel)",
            "type": "role",
            "required": false,
            "default": "",
            "description": "Rôle donné au membre lors d un /ban-sursis. Configure ce rôle pour qu il ne voie que le règlement. Requis pour le ban avec appel."
        }'::jsonb
        UNION ALL SELECT '{
            "key": "sursis_appeal_days",
            "label": "Délai d appel avant ban définitif (jours)",
            "type": "number",
            "required": false,
            "default": "7",
            "description": "Nombre de jours laissés au membre pour contester avant le bannissement automatique."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'moderation-bot';
