-- audit-bot — restauration du schema complet apres regression mig 210.
--
-- 3 channels que le code utilise mais qui n'etaient pas exposes :
--   - anomaly_channel_id : alertes urgentes mass_ban/delete/role
--   - join_leave_channel_id : embed join/leave
--   - profile_edit_channel_id : nickname/avatar/banner
-- Sans ces cles, tout retombait sur log_channel_id (fallback).
--
-- Schema reorganise en sections logiques (toggles + numbers + channels).

UPDATE bot_definitions SET
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active les logs d audit avances + cache Redis pour les watched users."},

        {"key": "log_channel_id", "label": "Salon de logs (general / fallback)", "type": "channel", "required": false, "description": "Salon de logs par defaut. Utilise comme fallback quand un salon plus specifique (anomaly/join_leave/profile_edit) n est pas configure.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "anomaly_channel_id", "label": "Salon anomalies", "type": "channel", "required": false, "description": "Salon des alertes urgentes (mass_ban, mass_delete, mass_role_change). Si vide -> log_channel_id.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "join_leave_channel_id", "label": "Salon joins / leaves", "type": "channel", "required": false, "description": "Salon ou poster les arrivees / departs de membres. Si vide -> log_channel_id.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "profile_edit_channel_id", "label": "Salon modifications profil", "type": "channel", "required": false, "description": "Nickname, avatar, banner, pseudo Discord global. Si vide -> log_channel_id.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "message_cache_size", "label": "Taille cache messages", "type": "number", "required": false, "default": "10000", "min": 100, "max": 100000, "description": "Buffer in-memory des messages recents pour les detections d anomalie. Per-guild override applique au prochain redemarrage du bot.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "audit_cache_refresh_interval", "label": "Refresh cache watched users", "type": "number", "required": false, "default": "60", "min": 10, "max": 3600, "unit": "s", "description": "Frequence de sync Redis -> bot du cache des utilisateurs surveilles.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "audit_sync_interval", "label": "Worker : intervalle sync audit Discord", "type": "number", "required": false, "default": "300", "min": 60, "max": 3600, "unit": "s", "description": "Frequence de polling des audit logs Discord via l API REST (rattrape les events rates en gateway).", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "anomaly_enabled", "label": "Detection d''anomalies", "type": "boolean", "required": false, "default": "true", "description": "Detecte les comportements en rafale (mass ban/delete/role change).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "anomaly_mass_ban_threshold", "label": "Seuil mass ban (par 60s)", "type": "number", "required": false, "default": "5", "min": 1, "max": 100, "depends_on": {"key": "anomaly_enabled", "equals": "true"}},
        {"key": "anomaly_mass_delete_threshold", "label": "Seuil mass delete (par 60s)", "type": "number", "required": false, "default": "20", "min": 1, "max": 1000, "depends_on": {"key": "anomaly_enabled", "equals": "true"}},
        {"key": "anomaly_mass_role_threshold", "label": "Seuil mass role change (par 60s)", "type": "number", "required": false, "default": "10", "min": 1, "max": 100, "depends_on": {"key": "anomaly_enabled", "equals": "true"}},

        {"key": "weekly_report_enabled", "label": "Rapport hebdomadaire (/audit stats)", "type": "boolean", "required": false, "default": "true", "description": "Active la commande /audit stats qui affiche un recap hebdo.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'audit-bot';
