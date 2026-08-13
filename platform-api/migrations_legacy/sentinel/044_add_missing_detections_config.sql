-- Migration 044 : Ajoute les clés de config pour les détections manquantes de l'automod-bot
-- (emoji spam, mentions excessives, fichiers suspects)

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "emoji_spam_enabled", "label": "Détection spam d''emojis", "type": "boolean", "required": false, "default": "true"},
  {"key": "emoji_spam_max", "label": "Nombre max d''emojis par message", "type": "number", "required": false, "default": "10"},
  {"key": "mentions_enabled", "label": "Détection mentions excessives", "type": "boolean", "required": false, "default": "true"},
  {"key": "mentions_max", "label": "Nombre max de mentions par message", "type": "number", "required": false, "default": "5"},
  {"key": "suspicious_files_enabled", "label": "Détection fichiers suspects", "type": "boolean", "required": false, "default": "true"},
  {"key": "suspicious_file_extensions", "label": "Extensions suspectes supplémentaires (CSV)", "type": "text", "required": false, "default": ""}
]'::jsonb
WHERE bot_name = 'automod-bot';
