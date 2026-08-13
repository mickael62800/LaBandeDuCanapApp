-- Audit-bot — log des commandes admin / moderateur.
--
-- Poste une ligne « X a utilise /commande » dans un salon dedie et
-- parametrable, uniquement pour les commandes admin/moderateur (automod,
-- moderation, securite, nettoyage, audit, rotation, *-setup, *-admin...).
-- Opt-in explicite : toggle + salon (aucun fallback sur log_channel_id).

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' NOT IN ('command_log_enabled', 'command_log_channel_id')
        UNION ALL SELECT '{
            "key": "command_log_enabled",
            "label": "Log des commandes admin/moderateur",
            "type": "boolean",
            "required": false,
            "default": "false",
            "description": "Poste une ligne quand une commande admin/moderateur est utilisee."
        }'::jsonb
        UNION ALL SELECT '{
            "key": "command_log_channel_id",
            "label": "Salon du log des commandes admin",
            "type": "channel",
            "required": false,
            "default": "",
            "description": "Salon ou poster le log des commandes admin/moderateur. Requis pour activer le log."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'audit-bot';
