INSERT INTO bot_definitions (bot_name, display_name, description, config_schema) VALUES
('moderation-worker', 'Worker Moderation', 'Regeneration des points de conduite, nettoyage bans et propositions de ban', '[
  {"key": "conduct_regen_interval", "label": "Intervalle regen conduite (heures)", "type": "number", "required": false, "default": "1"},
  {"key": "ban_cleanup_interval", "label": "Intervalle nettoyage bans (minutes)", "type": "number", "required": false, "default": "1"},
  {"key": "sync_ban_proposals_interval", "label": "Intervalle sync propositions ban (minutes)", "type": "number", "required": false, "default": "2"}
]'::jsonb),
('analytics-worker', 'Worker Analytics', 'Snapshots quotidiens et horaires pour les graphiques du dashboard', '[
  {"key": "daily_snapshot_interval", "label": "Intervalle snapshot quotidien (heures)", "type": "number", "required": false, "default": "1"},
  {"key": "hourly_snapshot_interval", "label": "Intervalle snapshot horaire (heures)", "type": "number", "required": false, "default": "1"}
]'::jsonb)
ON CONFLICT (bot_name) DO UPDATE SET
  display_name = EXCLUDED.display_name,
  description = EXCLUDED.description,
  config_schema = EXCLUDED.config_schema;
