-- Phase 8 : ajout des toggles review_mode au schema automod-bot.
-- Chaque detection peut etre en mode review (carte moderateur) ou auto (action directe).

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "ai_review_mode", "label": "Mode review IA (insultes, spam, liens, phishing)", "type": "boolean", "required": false, "default": "true"},
  {"key": "flood_review_mode", "label": "Mode review flood", "type": "boolean", "required": false, "default": "true"},
  {"key": "caps_review_mode", "label": "Mode review majuscules", "type": "boolean", "required": false, "default": "true"},
  {"key": "files_review_mode", "label": "Mode review fichiers suspects", "type": "boolean", "required": false, "default": "true"}
]'::jsonb
WHERE bot_name = 'automod-bot';
