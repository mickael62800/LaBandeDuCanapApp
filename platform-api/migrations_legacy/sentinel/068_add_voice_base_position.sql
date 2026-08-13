-- Ajout du parametre voice_base_position au config_schema du voice-bot
-- Permet de choisir la position de depart des salons temporaires dans Discord
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "voice_base_position", "label": "Position de depart des salons temporaires (index Discord)", "type": "number", "required": false, "default": ""}
]'::jsonb
WHERE bot_name = 'voice-bot';
