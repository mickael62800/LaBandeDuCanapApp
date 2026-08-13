-- Migration 115 : voice-bot — ajout du salon createur "game"
--
-- Nouveau type de salon temporaire dedie aux sessions de jeu.
-- Caracteristiques :
--   - user_limit = 10 (configurable apres creation par l'owner)
--   - visibilite : visible a tous
--   - queue activee automatiquement a la creation (personne ne peut
--     rejoindre sans passer par la file d'attente)
--   - meme panel admin que les salons public/prive
--
-- Compat : le champ est optionnel. Si non configure, le bot ne gere
-- simplement pas les hubs "game".

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "game_creator_channel_id", "label": "Salon createur \"game\" (salons de jeu, queue obligatoire)", "type": "channel", "required": false, "default": ""}
]'::jsonb
WHERE bot_name = 'voice-bot';
