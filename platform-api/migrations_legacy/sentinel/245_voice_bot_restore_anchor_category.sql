-- voice-bot — restauration de voice_anchor_category_id dans le schema.
--
-- Meme regression que les salons creators (cf. 244) : la migration 239 a
-- reecrit le config_schema from scratch et a oublie voice_anchor_category_id,
-- pourtant toujours lu par le code (channel_lifecycle.rs:76-93 : le salon
-- temporaire est cree DANS cette categorie). Sans le champ, impossible de
-- choisir la categorie d ancrage depuis la page Composants -> les salons
-- temp se creent a la racine (tout en haut du serveur).
--
-- Type "category" (et non "channel") pour n afficher que les categories
-- Discord dans le selecteur (cf. 236).

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "voice_anchor_category_id", "label": "Categorie des salons temporaires", "type": "category", "required": false, "description": "Les salons vocaux temporaires seront crees dans cette categorie (en bas). Vide = racine du serveur.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'voice-bot'
  AND NOT EXISTS (
      SELECT 1 FROM jsonb_array_elements(config_schema) e
       WHERE e->>'key' = 'voice_anchor_category_id'
  );
