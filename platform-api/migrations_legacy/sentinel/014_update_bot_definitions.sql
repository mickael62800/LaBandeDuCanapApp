-- Mise a jour des definitions des bots avec tous les parametres configurables

UPDATE bot_definitions SET config_schema = '[
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
    {"key": "flood_max_messages", "label": "Seuil de flood (messages)", "type": "number", "required": false, "default": "5"},
    {"key": "flood_window_secs", "label": "Fenetre de flood (secondes)", "type": "number", "required": false, "default": "10"},
    {"key": "mute_duration_secs", "label": "Duree du mute (secondes)", "type": "number", "required": false, "default": "600"},
    {"key": "ignored_roles", "label": "Roles ignores (IDs separes par des virgules)", "type": "text", "required": false}
]' WHERE bot_name = 'automod-bot';

UPDATE bot_definitions SET config_schema = '[
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
    {"key": "default_mute_duration_mins", "label": "Duree mute par defaut (minutes)", "type": "number", "required": false, "default": "60"},
    {"key": "ban_delete_days", "label": "Jours de messages supprimes au ban", "type": "number", "required": false, "default": "1"},
    {"key": "warn_threshold_to_mute", "label": "Warns avant auto-mute (0 = desactive)", "type": "number", "required": false, "default": "0"}
]' WHERE bot_name = 'moderation-bot';

UPDATE bot_definitions SET config_schema = '[
    {"key": "alert_channel_id", "label": "Salon d alertes", "type": "channel", "required": false},
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
    {"key": "raid_join_threshold", "label": "Seuil raid (nombre de joins)", "type": "number", "required": false, "default": "10"},
    {"key": "raid_join_window_secs", "label": "Fenetre raid (secondes)", "type": "number", "required": false, "default": "10"},
    {"key": "min_account_age_secs", "label": "Age minimum du compte (secondes)", "type": "number", "required": false, "default": "86400"}
]' WHERE bot_name = 'security-bot';

UPDATE bot_definitions SET config_schema = '[
    {"key": "tracking_enabled", "label": "Suivi actif", "type": "boolean", "required": false, "default": "true"},
    {"key": "leaderboard_default_size", "label": "Taille classement par defaut", "type": "number", "required": false, "default": "10"}
]' WHERE bot_name = 'stats-bot';

UPDATE bot_definitions SET config_schema = '[
    {"key": "assistance_channel_id", "label": "Salon d assistance", "type": "channel", "required": true},
    {"key": "admin_role_id", "label": "Role Administrateur", "type": "role", "required": true},
    {"key": "moderator_role_id", "label": "Role Moderateur", "type": "role", "required": true},
    {"key": "max_open_per_user", "label": "Limite tickets ouverts par utilisateur (0 = illimite)", "type": "number", "required": false, "default": "0"}
]' WHERE bot_name = 'ticket-bot';

UPDATE bot_definitions SET config_schema = '[
    {"key": "public_creator_channel_id", "label": "Salon createur public", "type": "channel", "required": true},
    {"key": "private_creator_channel_id", "label": "Salon createur prive", "type": "channel", "required": true},
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
    {"key": "cooldown_secs", "label": "Cooldown creation (secondes)", "type": "number", "required": false, "default": "5"},
    {"key": "vote_timeout_secs", "label": "Timeout vote kick (secondes)", "type": "number", "required": false, "default": "60"}
]' WHERE bot_name = 'voice-bot';
