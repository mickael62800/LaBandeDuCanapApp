-- Phase composants — Fusion `audit-cache-worker` dans `audit-bot`.
--
-- audit-bot avait 8 cles metier (anomalies, seuils, weekly report).
-- audit-cache-worker avait 1 cle infra (audit_cache_refresh_interval)
-- + un `enabled` redondant avec celui d'audit-bot. On merge les deux.

-- 1) Configs : remet le bot_name='audit-bot' (mig 204 avait
-- renomme audit-cache-worker -> audit_cache).
DELETE FROM bot_guild_config wkr
    WHERE wkr.bot_name = 'audit_cache'
      AND EXISTS (
          SELECT 1 FROM bot_guild_config m
           WHERE m.bot_name = 'audit-bot'
             AND m.guild_id = wkr.guild_id
             AND m.config_key = wkr.config_key
      );
UPDATE bot_guild_config SET bot_name = 'audit-bot'
    WHERE bot_name = 'audit_cache';

-- 2) Schema fusionne avec depends_on en cascade.
UPDATE bot_definitions SET
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active les logs d audit avances + cache Redis pour les watched users."},
        {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "message_cache_size", "label": "Taille cache messages", "type": "number", "required": false, "default": "10000", "min": 100, "max": 100000, "description": "Buffer in-memory des messages recents pour les detections d anomalie.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "audit_cache_refresh_interval", "label": "Refresh cache watched users", "type": "number", "required": false, "default": "60", "min": 10, "max": 3600, "unit": "s", "description": "Frequence de sync Redis -> bot du cache des utilisateurs surveilles.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "anomaly_enabled", "label": "Detection d''anomalies", "type": "boolean", "required": false, "default": "true", "description": "Detecte les comportements en rafale (mass ban/delete/role change).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "anomaly_mass_ban_threshold", "label": "Seuil mass ban (par 60s)", "type": "number", "required": false, "default": "5", "min": 1, "max": 100, "depends_on": {"key": "anomaly_enabled", "equals": "true"}},
        {"key": "anomaly_mass_delete_threshold", "label": "Seuil mass delete (par 60s)", "type": "number", "required": false, "default": "20", "min": 1, "max": 1000, "depends_on": {"key": "anomaly_enabled", "equals": "true"}},
        {"key": "anomaly_mass_role_threshold", "label": "Seuil mass role change (par 60s)", "type": "number", "required": false, "default": "10", "min": 1, "max": 100, "depends_on": {"key": "anomaly_enabled", "equals": "true"}},

        {"key": "weekly_report_enabled", "label": "Rapport hebdomadaire", "type": "boolean", "required": false, "default": "true", "description": "Envoi auto d un recap hebdo dans le salon de logs.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'audit-bot';

DELETE FROM bot_definitions WHERE bot_name = 'audit-cache-worker';
