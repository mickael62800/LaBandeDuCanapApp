-- Retire le mot "Bot" des display_names et descriptions visibles dans l'UI.
-- Post-fusion : les anciens "bots" sont des modules du binaire unifie sentinel-bot.
-- Note : bot_name (identifiant technique) reste inchange pour ne pas casser le schema.

UPDATE bot_definitions SET display_name = 'Image'   WHERE bot_name = 'image-bot'   AND display_name = 'Image Bot';
UPDATE bot_definitions SET display_name = 'Games'   WHERE bot_name = 'game-bot'    AND display_name = 'Game Bot';
UPDATE bot_definitions SET display_name = 'Welcome' WHERE bot_name = 'welcome-bot' AND display_name = 'Welcome Bot';

-- Descriptions : "Bot de ..." → "Module de ..."
UPDATE bot_definitions
SET description = 'Module de detection d''images NSFW et produits illicites via inference IA (ONNX)'
WHERE bot_name = 'image-bot'
  AND description LIKE 'Bot de detection%';

-- Worker monitoring : "bots et workers" → "modules et workers"
UPDATE bot_definitions
SET description = REPLACE(description, 'bots et workers', 'modules et workers')
WHERE description LIKE '%bots et workers%';

-- Remplace "le bot xxx" → "le module xxx" dans les descriptions de config_schema.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE
            WHEN elem ? 'description' AND elem->>'description' LIKE '%le bot %'
                THEN jsonb_set(elem, '{description}', to_jsonb(REPLACE(elem->>'description', 'le bot ', 'le module ')))
            ELSE elem
        END
    )
    FROM jsonb_array_elements(config_schema) AS elem
)
WHERE config_schema::text LIKE '%le bot %';
