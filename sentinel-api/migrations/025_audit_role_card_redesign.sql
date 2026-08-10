-- Aligne la carte vivante des rôles sur la nouvelle fenêtre par défaut de
-- deux minutes, pour les nouvelles installations comme pour les guildes qui
-- utilisaient encore l'ancien défaut de cinq minutes.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE
            WHEN elem->>'key' = 'role_log_window_secs' THEN
                elem || '{
                    "default": "120",
                    "description": "Duree pendant laquelle la carte de changement de roles reste active et se met a jour (fenetre glissante). Evite le spam d une carte par role. Defaut 120 (2 min)."
                }'::jsonb
            ELSE elem
        END
    )
    FROM jsonb_array_elements(config_schema) AS elem
)
WHERE bot_name = 'audit-bot';

UPDATE bot_guild_config
SET config_value = '120', updated_at = now()
WHERE bot_name = 'audit-bot'
  AND config_key = 'role_log_window_secs'
  AND config_value = '300';
