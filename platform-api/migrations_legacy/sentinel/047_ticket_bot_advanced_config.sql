-- Migration 047 : Ajoute les cles de config pour les features avancees du ticket-bot
-- (SLA, satisfaction, templates, FAQ, transcript format, escalade)

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "sla_first_response_minutes", "label": "SLA premiere reponse (minutes, 0=desactive)", "type": "number", "required": false, "default": "30"},
  {"key": "sla_escalation_minutes", "label": "Delai escalade auto (minutes, 0=desactive)", "type": "number", "required": false, "default": "60"},
  {"key": "satisfaction_enabled", "label": "Sondage satisfaction apres fermeture", "type": "boolean", "required": false, "default": "true"},
  {"key": "response_templates", "label": "Templates de reponses (format: label|contenu, un par ligne)", "type": "text", "required": false, "default": ""},
  {"key": "faq_entries", "label": "FAQ (format: question|reponse, une par ligne)", "type": "text", "required": false, "default": ""},
  {"key": "transcript_format", "label": "Format transcript (text, markdown, html)", "type": "text", "required": false, "default": "text"}
]'::jsonb
WHERE bot_name = 'ticket-bot';
