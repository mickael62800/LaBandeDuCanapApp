-- Audit-bot — salon de logs dédié aux messages (édition / suppression).
--
-- Le bot poste désormais un embed dans ce salon lorsqu'un message est édité
-- (avant / après) ou supprimé. Si non défini, fallback sur log_channel_id.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'message_log_channel_id'
        UNION ALL SELECT '{
            "key": "message_log_channel_id",
            "label": "Salon de logs des messages (édition / suppression)",
            "type": "channel",
            "required": false,
            "default": "",
            "description": "Salon où le bot poste un embed quand un message est édité (avant/après) ou supprimé. Si vide, utilise log_channel_id."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'audit-bot';
