-- Permet a l'admin de designer un serveur-ressources separe pour stocker
-- les emojis custom utilises par les jeux. Si vide ou absent, l'upload
-- se fait dans la guild courante (celle en cours de gestion).

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "emoji_host_guild_id", "label": "Serveur hote pour les emojis (ID)", "type": "text", "required": false, "description": "ID Discord d''un serveur-ressources pour stocker les emojis custom. Si vide, utilise le serveur courant. Le bot doit y etre present et avoir la permission MANAGE_GUILD_EXPRESSIONS."}
]'::jsonb
WHERE bot_name = 'game-bot'
  AND NOT (config_schema @> '[{"key": "emoji_host_guild_id"}]'::jsonb);
