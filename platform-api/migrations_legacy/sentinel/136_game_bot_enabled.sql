-- Ajoute la cle "enabled" au config_schema de game-bot.
-- Sans ce toggle, l UI ne permet pas d activer/desactiver le module.
-- Default true = module actif par defaut (retro-compatible).

UPDATE bot_definitions
SET config_schema = '[
    {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active ou desactive le module jeux (commandes /game, /game-admin et mentions #Jeu)."},
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
    {"key": "max_games", "label": "Nombre max de jeux par serveur (0 = illimite)", "type": "number", "required": false, "default": "50"},
    {"key": "role_color", "label": "Couleur des roles jeux (hex sans #)", "type": "text", "required": false, "default": "3498db"},
    {"key": "emoji_host_guild_id", "label": "ID du serveur host des emojis", "type": "text", "required": false, "description": "Serveur Discord ou le bot upload les emojis custom. Vide = serveur courant."}
]'::jsonb
WHERE bot_name = 'game-bot';
