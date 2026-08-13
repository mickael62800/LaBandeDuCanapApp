-- Convocation (/call) : le reglage call_category_id etait de type "channel"
-- (le dashboard listait des salons au lieu de categories) -> impossible de
-- choisir une categorie, donc le salon de convocation se creait "n'importe ou".
-- On bascule le champ en type "category" -> selecteur de categories dans le web.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE
            WHEN elem->>'key' = 'call_category_id'
                THEN jsonb_set(elem, '{type}', '"category"')
            ELSE elem
        END
    )
    FROM jsonb_array_elements(config_schema) AS elem
)
WHERE bot_name = 'moderation-bot'
  AND config_schema @> '[{"key": "call_category_id"}]'::jsonb;
