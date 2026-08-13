-- Ajout des definitions pour les 2 nouveaux workers : cache-worker et cleanup-worker.
-- Permet de les configurer depuis l'application bureau (BotConfigPage / WorkerConfigPage).

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema) VALUES
('cache-worker', 'Worker Cache', 'Pre-calcul des donnees analytics et dashboard dans Redis pour des reponses instantanees', '[
  {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true"},
  {"key": "analytics_cache_refresh", "label": "Rafraichissement cache analytics (secondes)", "type": "number", "required": false, "default": "300"},
  {"key": "dashboard_cache_refresh", "label": "Rafraichissement cache dashboard (secondes)", "type": "number", "required": false, "default": "600"},
  {"key": "voice_stats_cache_refresh", "label": "Rafraichissement stats vocales (secondes)", "type": "number", "required": false, "default": "3600"}
]'::jsonb),
('cleanup-worker', 'Worker Nettoyage', 'Nettoyage automatique des donnees anciennes et maintenance de la base de donnees', '[
  {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true"},
  {"key": "voice_sessions_retention_days", "label": "Retention sessions vocales (jours)", "type": "number", "required": false, "default": "90"},
  {"key": "logs_retention_days", "label": "Retention logs (jours)", "type": "number", "required": false, "default": "30"},
  {"key": "closed_tickets_retention_days", "label": "Retention tickets fermes (jours)", "type": "number", "required": false, "default": "180"},
  {"key": "cleanup_interval_hours", "label": "Intervalle nettoyage (heures)", "type": "number", "required": false, "default": "1"},
  {"key": "vacuum_enabled", "label": "VACUUM automatique", "type": "boolean", "required": false, "default": "true"},
  {"key": "vacuum_interval_hours", "label": "Intervalle VACUUM (heures)", "type": "number", "required": false, "default": "24"}
]'::jsonb),
('monitoring-worker', 'Worker Monitoring', 'Surveillance de la sante des bots et workers, alertes en temps reel', '[
  {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true"},
  {"key": "check_interval", "label": "Intervalle de verification (secondes)", "type": "number", "required": false, "default": "30"}
]'::jsonb)
ON CONFLICT (bot_name) DO UPDATE SET
  display_name = EXCLUDED.display_name,
  description = EXCLUDED.description,
  config_schema = EXCLUDED.config_schema;
