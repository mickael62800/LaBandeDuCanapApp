-- Migration 045 : Ajoute les cles de config pour les features avancees du security-bot
-- (lockdown, smart captcha, alt detection, raid pattern analysis)

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "lockdown_enabled", "label": "Lockdown auto (desactive envoi messages)", "type": "boolean", "required": false, "default": "false"},
  {"key": "lockdown_duration_secs", "label": "Duree max du lockdown (secondes)", "type": "number", "required": false, "default": "300"},
  {"key": "captcha_type", "label": "Type de captcha (button, math)", "type": "text", "required": false, "default": "button"},
  {"key": "alt_detection_enabled", "label": "Detection de comptes alt", "type": "boolean", "required": false, "default": "false"},
  {"key": "alt_retention_secs", "label": "Retention bans pour detection alt (secondes)", "type": "number", "required": false, "default": "604800"},
  {"key": "alt_name_distance", "label": "Seuil distance Levenshtein (noms alt)", "type": "number", "required": false, "default": "2"},
  {"key": "raid_pattern_enabled", "label": "Detection patterns de raid avancee", "type": "boolean", "required": false, "default": "true"},
  {"key": "raid_pattern_score_threshold", "label": "Score seuil pattern raid (0-100)", "type": "number", "required": false, "default": "60"}
]'::jsonb
WHERE bot_name = 'security-bot';
