-- Phase composants — `analytics-worker` n'a pas de module bot. Renomme
-- en `analytics` (mig 204 a deja renomme les rows) avec cascade depends_on.

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'analytics',
    'Analytics & snapshots',
    'Genere les snapshots horaires et quotidiens (messages, voice, joins, sanctions) qui alimentent les graphiques du dashboard et le rapport mensuel.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Si OFF : pas de snapshots, les graphiques du dashboard restent figes."},
        {"key": "track_voice_stats", "label": "Tracker stats vocales", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "track_message_stats", "label": "Tracker stats messages", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "hourly_snapshot_interval", "label": "Intervalle snapshot horaire", "type": "number", "required": false, "default": "3600", "min": 600, "max": 86400, "unit": "s", "description": "Frequence des snapshots horaires (granularite des graphiques recents).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "daily_snapshot_interval", "label": "Intervalle snapshot journalier", "type": "number", "required": false, "default": "86400", "min": 3600, "max": 604800, "unit": "s", "description": "Frequence des snapshots journaliers (donnees long terme).", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "data_retention_days", "label": "Retention des donnees", "type": "number", "required": false, "default": "90", "min": 0, "max": 3650, "unit": "j", "description": "Apres combien de jours les snapshots sont supprimes. 0 = illimite.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "top_users_count", "label": "Top utilisateurs (taille)", "type": "number", "required": false, "default": "10", "min": 1, "max": 100, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "export_format", "label": "Format d''export", "type": "enum", "required": false, "default": "json", "options": [{"value": "json", "label": "JSON"}, {"value": "csv", "label": "CSV"}], "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "monthly_report_enabled", "label": "Rapport mensuel auto", "type": "boolean", "required": false, "default": "true", "description": "Envoi automatique d un recap mensuel dans le salon configure.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "monthly_report_channel_id", "label": "Salon rapport mensuel", "type": "channel", "required": false, "depends_on": {"key": "monthly_report_enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;

DELETE FROM bot_definitions WHERE bot_name = 'analytics-worker';
