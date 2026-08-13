-- Configuration complete pour automod-bot
UPDATE bot_definitions SET config_schema = '[
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
    {"key": "ignored_roles", "label": "Roles ignores (IDs separes par des virgules)", "type": "text", "required": false},
    {"key": "ignored_channels", "label": "Salons ignores (IDs separes par des virgules)", "type": "text", "required": false},

    {"key": "flood_max_messages", "label": "Seuil de flood (nombre de messages)", "type": "number", "required": false, "default": "5"},
    {"key": "flood_window_secs", "label": "Fenetre de flood (secondes)", "type": "number", "required": false, "default": "10"},
    {"key": "mute_duration_secs", "label": "Duree du mute (secondes)", "type": "number", "required": false, "default": "600"},

    {"key": "spam_detection_enabled", "label": "Detection spam activee", "type": "boolean", "required": false, "default": "true"},
    {"key": "spam_repeat_char_threshold", "label": "Seuil caracteres repetes (ex: aaaaaa)", "type": "number", "required": false, "default": "6"},
    {"key": "spam_repeat_word_threshold", "label": "Seuil mots repetes (ex: lol lol lol)", "type": "number", "required": false, "default": "5"},

    {"key": "caps_warning_enabled", "label": "Avertissement majuscules excessives", "type": "boolean", "required": false, "default": "true"},
    {"key": "caps_threshold_chars", "label": "Seuil caracteres majuscules", "type": "number", "required": false, "default": "8"},

    {"key": "insult_detection_enabled", "label": "Detection insultes activee", "type": "boolean", "required": false, "default": "true"},
    {"key": "insult_custom_words", "label": "Mots interdits supplementaires (separes par des virgules)", "type": "text", "required": false},

    {"key": "link_detection_enabled", "label": "Detection liens activee", "type": "boolean", "required": false, "default": "true"},
    {"key": "allow_discord_invites", "label": "Autoriser les invitations Discord", "type": "boolean", "required": false, "default": "false"},
    {"key": "allowed_domains", "label": "Domaines autorises (separes par des virgules)", "type": "text", "required": false},

    {"key": "phishing_detection_enabled", "label": "Detection phishing activee", "type": "boolean", "required": false, "default": "true"},
    {"key": "phishing_extra_whitelist", "label": "Domaines de confiance supplementaires (separes par des virgules)", "type": "text", "required": false},

    {"key": "color_warn", "label": "Couleur embed avertissement (hex sans #)", "type": "text", "required": false, "default": "f59e0b"},
    {"key": "color_delete", "label": "Couleur embed suppression (hex sans #)", "type": "text", "required": false, "default": "f97316"},
    {"key": "color_mute", "label": "Couleur embed mute (hex sans #)", "type": "text", "required": false, "default": "ef4444"},
    {"key": "color_ban", "label": "Couleur embed ban (hex sans #)", "type": "text", "required": false, "default": "dc2626"}
]' WHERE bot_name = 'automod-bot';
