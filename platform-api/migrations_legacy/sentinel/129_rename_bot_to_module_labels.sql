-- Renomme les labels "Bot actif" → "Module actif" dans tous les config_schema.
-- Post-fusion : les 15 anciens "bots" sont maintenant des modules du binaire
-- unifie sentinel-bot. Le label UI doit refleter cette terminologie.
-- Les workers gardent leur label "Worker actif".

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE
            WHEN elem->>'label' = 'Bot actif' THEN elem || '{"label": "Module actif"}'::jsonb
            ELSE elem
        END
    )
    FROM jsonb_array_elements(config_schema) AS elem
)
WHERE config_schema @> '[{"label": "Bot actif"}]'::jsonb;
