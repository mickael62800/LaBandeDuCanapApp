-- Automod vote — fil de discussion attache a la carte de vote.
--
-- Si actif, le bot ouvre automatiquement un fil (thread) sur la carte de
-- vote pour que les moderateurs puissent en discuter avant/pendant le vote.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "vote_thread_enabled", "label": "Fil de discussion sur la carte", "type": "boolean", "required": false, "default": "true", "description": "Ouvre automatiquement un fil de discussion attache a chaque carte de vote pour que les moderateurs en debattent.", "depends_on": {"key": "vote_enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key": "vote_thread_enabled"}]'::jsonb);
