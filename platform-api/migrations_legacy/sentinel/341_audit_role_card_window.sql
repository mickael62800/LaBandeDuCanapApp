-- Audit-bot — carte de changement de roles « vivante » (anti-spam).
-- Au lieu d'une carte par role, une seule carte par membre reste active pendant
-- cette fenetre glissante et se met a jour avec l'historique des roles.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'role_log_window_secs'
        UNION ALL SELECT '{
            "key": "role_log_window_secs",
            "label": "Fenetre carte roles (secondes)",
            "type": "number",
            "required": false,
            "default": "300",
            "description": "Duree pendant laquelle la carte de changement de roles reste active et se met a jour (fenetre glissante). Evite le spam d une carte par role. Defaut 300 (5 min)."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'audit-bot';
