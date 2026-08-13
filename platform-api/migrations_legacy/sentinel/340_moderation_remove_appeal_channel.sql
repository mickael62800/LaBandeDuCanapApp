-- Moderation-bot — l'appel de sanction passe UNIQUEMENT par une categorie
-- (appeal_category_id) : le bot cree un salon prive sous cette categorie. On
-- retire donc l'ancienne cle appeal_channel_id (notification par salon), qui
-- faisait doublon et pretait a confusion.

UPDATE bot_definitions
SET config_schema = (
    SELECT COALESCE(jsonb_agg(elem), '[]'::jsonb)
    FROM jsonb_array_elements(config_schema) AS elem
    WHERE elem->>'key' <> 'appeal_channel_id'
)
WHERE bot_name = 'moderation-bot';
