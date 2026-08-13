-- Migration 048 : Ajoute les cles de config pour les features avancees de l'automod-bot
-- (mode nuit, detection unicode, slowmode adaptatif)

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "night_mode_enabled", "label": "Mode nuit (seuils plus stricts)", "type": "boolean", "required": false, "default": "false"},
  {"key": "night_start_hour", "label": "Heure debut mode nuit (UTC, 0-23)", "type": "number", "required": false, "default": "22"},
  {"key": "night_end_hour", "label": "Heure fin mode nuit (UTC, 0-23)", "type": "number", "required": false, "default": "8"},
  {"key": "unicode_detection_enabled", "label": "Detection abus Unicode (zalgo, invisibles, homoglyphes)", "type": "boolean", "required": false, "default": "true"},
  {"key": "unicode_max_combining", "label": "Max combining characters par lettre (zalgo)", "type": "number", "required": false, "default": "3"},
  {"key": "unicode_max_invisible", "label": "Max caracteres invisibles par message", "type": "number", "required": false, "default": "5"},
  {"key": "adaptive_slowmode_enabled", "label": "Slowmode adaptatif automatique", "type": "boolean", "required": false, "default": "false"},
  {"key": "adaptive_slowmode_threshold", "label": "Seuil messages par 30s pour activation", "type": "number", "required": false, "default": "15"},
  {"key": "adaptive_slowmode_seconds", "label": "Secondes de slowmode quand active", "type": "number", "required": false, "default": "5"}
]'::jsonb
WHERE bot_name = 'automod-bot';
