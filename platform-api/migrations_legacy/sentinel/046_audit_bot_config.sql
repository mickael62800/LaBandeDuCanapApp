-- Migration 046 : Creer l'entree bot_definitions pour audit-bot
-- avec config_schema pour les features avancees

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
  'audit-bot',
  'Audit Bot',
  'Bot d''audit — logs avances des evenements serveur',
  '[
    {"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"},
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false, "default": ""},
    {"key": "message_cache_size", "label": "Taille cache messages", "type": "number", "required": false, "default": "10000"},
    {"key": "anomaly_enabled", "label": "Detection d''anomalies", "type": "boolean", "required": false, "default": "true"},
    {"key": "anomaly_mass_ban_threshold", "label": "Seuil mass ban (en 60s)", "type": "number", "required": false, "default": "5"},
    {"key": "anomaly_mass_delete_threshold", "label": "Seuil mass delete (en 60s)", "type": "number", "required": false, "default": "20"},
    {"key": "anomaly_mass_role_threshold", "label": "Seuil mass role change (en 60s)", "type": "number", "required": false, "default": "10"},
    {"key": "weekly_report_enabled", "label": "Rapport hebdomadaire", "type": "boolean", "required": false, "default": "true"}
  ]'::jsonb
)
ON CONFLICT (bot_name) DO UPDATE SET
  config_schema = EXCLUDED.config_schema,
  display_name = EXCLUDED.display_name,
  description = EXCLUDED.description;
