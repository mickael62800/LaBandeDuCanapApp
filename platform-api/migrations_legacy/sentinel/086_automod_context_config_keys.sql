-- Ajout des parametres de contexte conversationnel dans les definitions automod-bot
UPDATE bot_definitions SET config_schema = config_schema::jsonb || '[
    {"key": "context_max_messages", "label": "Messages de contexte (nombre)", "type": "number", "required": false, "default": "3"},
    {"key": "context_max_chars", "label": "Caracteres max par message de contexte", "type": "number", "required": false, "default": "200"}
]'::jsonb WHERE bot_name = 'automod-bot';
