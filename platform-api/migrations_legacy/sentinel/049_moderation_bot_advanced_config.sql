-- Migration 049 : Ajoute les cles de config pour les features avancees du moderation-bot
-- (mode apprenti, templates raisons, appel sanction, convocation)

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "apprentice_role_id", "label": "Role moderateur apprenti (actions en attente)", "type": "role", "required": false, "default": ""},
  {"key": "reason_templates", "label": "Templates de raisons (format: label|raison, un par ligne)", "type": "text", "required": false, "default": ""},
  {"key": "appeal_enabled", "label": "Bouton appel de sanction dans les DMs", "type": "boolean", "required": false, "default": "true"},
  {"key": "call_category_id", "label": "Categorie Discord pour les convocations", "type": "channel", "required": false, "default": ""}
]'::jsonb
WHERE bot_name = 'moderation-bot';
