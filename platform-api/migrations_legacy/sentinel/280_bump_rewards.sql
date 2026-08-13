-- Bump rewards : recompense des coins quand un membre fait /bump (Disboard),
-- avec une recompense GRADUEE selon le nombre de bumps de la semaine (plus on
-- bump dans la semaine, plus la recompense monte), + rappel apres le cooldown.

CREATE TABLE IF NOT EXISTS bump_events (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id      TEXT NOT NULL,
    user_id       TEXT NOT NULL,
    username      TEXT NOT NULL DEFAULT '',
    reward_coins  INTEGER NOT NULL DEFAULT 0,
    weekly_index  INTEGER NOT NULL DEFAULT 1,   -- Nieme bump de la semaine (>=1)
    bumped_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bump_events_user_week
    ON bump_events (guild_id, user_id, bumped_at DESC);

-- Etat par guild pour piloter le rappel apres cooldown (snapshot de la config
-- au moment du bump pour eviter de relire la config dans la boucle de rappel).
CREATE TABLE IF NOT EXISTS bump_guild_state (
    guild_id          TEXT PRIMARY KEY,
    channel_id        TEXT NOT NULL DEFAULT '',
    last_bump_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    cooldown_minutes  INTEGER NOT NULL DEFAULT 120,
    reminder_enabled  BOOLEAN NOT NULL DEFAULT TRUE,
    reminder_sent     BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Definition du bot pour la page Composants + config web.
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'bump-bot',
    'Bump Rewards',
    'Recompense des coins quand un membre fait /bump (Disboard), recompense graduee selon le nombre de bumps de la semaine, + rappel apres cooldown.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "false", "description": "Active la recompense de bump."},
        {"key": "bump_reward_base", "label": "Coins de base par bump", "type": "number", "required": false, "default": "100", "description": "Recompense du 1er bump de la semaine."},
        {"key": "bump_reward_step", "label": "Bonus par bump suppl. dans la semaine", "type": "number", "required": false, "default": "50", "description": "Ajoute par bump au-dela du 1er (recompense graduee)."},
        {"key": "bump_reward_max", "label": "Recompense maximale par bump", "type": "number", "required": false, "default": "500", "description": "Plafond de la recompense graduee."},
        {"key": "bump_cooldown_minutes", "label": "Cooldown du bump (minutes)", "type": "number", "required": false, "default": "120", "description": "Delai Disboard entre deux bumps (defaut 120)."},
        {"key": "bump_reminder_enabled", "label": "Rappel apres cooldown", "type": "boolean", "required": false, "default": "true", "description": "Poste un rappel dans le salon quand un nouveau bump est possible."},
        {"key": "bump_channel_id", "label": "Salon des bumps (annonce + rappel)", "type": "channel", "required": false, "default": "", "description": "Salon ou poster la confirmation de recompense et le rappel. Si vide, utilise le salon du bump."}
    ]'::jsonb
)
ON CONFLICT (bot_name) DO UPDATE
    SET display_name = EXCLUDED.display_name,
        description = EXCLUDED.description,
        config_schema = EXCLUDED.config_schema;
