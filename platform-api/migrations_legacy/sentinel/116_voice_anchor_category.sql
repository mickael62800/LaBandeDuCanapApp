-- Migration 116 : voice-bot — ajout de voice_anchor_category_id
--
-- Alternative plus intuitive a voice_base_position (index numerique).
-- L'utilisateur cree une categorie vide (ex: "=== Salons dynamiques ===")
-- et colle son ID ici. A chaque creation de salon temp, le bot lit la
-- position actuelle de cette categorie et place le nouveau salon juste
-- en dessous.
--
-- Avantages vs voice_base_position :
--   - pas besoin de calculer un index a la main
--   - si l'user deplace la categorie ancre, les salons suivent
--   - plus de confusion entre index "categorie" et index "salon"
--
-- Priorite : anchor > base_position > Discord default

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "voice_anchor_category_id", "label": "Categorie ancre (les salons temp apparaissent juste en dessous)", "type": "channel", "required": false, "default": ""}
]'::jsonb
WHERE bot_name = 'voice-bot';
