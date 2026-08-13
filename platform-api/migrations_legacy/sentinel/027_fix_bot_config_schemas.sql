-- Correction des config_schema pour aligner les cles avec le code des bots
-- Problemes corriges :
--   moderation-bot : ban_delete_days -> ban_delete_message_days, ajout max_mute_duration_secs
--   security-bot : ajout quarantine_enabled, quarantine_role_id, captcha_enabled, slowmode_seconds

UPDATE bot_definitions SET config_schema = '[
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
    {"key": "default_mute_duration_secs", "label": "Duree mute par defaut (secondes)", "type": "number", "required": false, "default": "600"},
    {"key": "max_mute_duration_secs", "label": "Duree max du mute (secondes)", "type": "number", "required": false, "default": "2419200"},
    {"key": "ban_delete_message_days", "label": "Jours de messages supprimes au ban", "type": "number", "required": false, "default": "1"},
    {"key": "warn_threshold_to_mute", "label": "Warns avant auto-mute (0 = desactive)", "type": "number", "required": false, "default": "0"}
]' WHERE bot_name = 'moderation-bot';

UPDATE bot_definitions SET config_schema = '[
    {"key": "alert_channel_id", "label": "Salon d alertes", "type": "channel", "required": false},
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
    {"key": "raid_join_threshold", "label": "Seuil raid (nombre de joins)", "type": "number", "required": false, "default": "10"},
    {"key": "raid_join_window_secs", "label": "Fenetre raid (secondes)", "type": "number", "required": false, "default": "10"},
    {"key": "min_account_age_secs", "label": "Age minimum du compte (secondes)", "type": "number", "required": false, "default": "86400"},
    {"key": "quarantine_enabled", "label": "Quarantaine activee", "type": "boolean", "required": false, "default": "false"},
    {"key": "quarantine_role_id", "label": "Role de quarantaine", "type": "role", "required": false},
    {"key": "captcha_enabled", "label": "Captcha active", "type": "boolean", "required": false, "default": "false"},
    {"key": "slowmode_seconds", "label": "Slowmode anti-raid (secondes, 0 = desactive)", "type": "number", "required": false, "default": "0"}
]' WHERE bot_name = 'security-bot';
