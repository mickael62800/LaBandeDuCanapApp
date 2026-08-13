-- Configuration complete pour voice-bot
UPDATE bot_definitions SET config_schema = '[
    {"key": "public_creator_channel_id", "label": "Salon createur public", "type": "channel", "required": true},
    {"key": "private_creator_channel_id", "label": "Salon createur prive", "type": "channel", "required": true},
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},

    {"key": "cooldown_secs", "label": "Cooldown creation de salon (secondes)", "type": "number", "required": false, "default": "5"},

    {"key": "flood_max_messages", "label": "Seuil anti-flood (nombre de messages)", "type": "number", "required": false, "default": "5"},
    {"key": "flood_window_secs", "label": "Fenetre anti-flood (secondes)", "type": "number", "required": false, "default": "5"},
    {"key": "flood_mute_duration_secs", "label": "Duree mute anti-flood (secondes)", "type": "number", "required": false, "default": "30"},

    {"key": "vote_timeout_secs", "label": "Duree du vote kick (secondes)", "type": "number", "required": false, "default": "60"},
    {"key": "vote_min_members", "label": "Membres minimum pour lancer un vote kick", "type": "number", "required": false, "default": "2"},
    {"key": "vote_majority_percent", "label": "Pourcentage de majorite pour le vote kick", "type": "number", "required": false, "default": "50"},

    {"key": "queue_user_limit", "label": "Limite de la file d attente", "type": "number", "required": false, "default": "99"},
    {"key": "queue_enabled_by_default", "label": "File d attente activee par defaut", "type": "boolean", "required": false, "default": "false"},

    {"key": "default_channel_name", "label": "Nom par defaut du salon (utiliser {user} pour le pseudo)", "type": "text", "required": false, "default": "Salon de {user}"},
    {"key": "default_member_limit", "label": "Limite de membres par defaut (0 = illimite)", "type": "number", "required": false, "default": "0"},

    {"key": "auto_delete_empty", "label": "Supprimer automatiquement les salons vides", "type": "boolean", "required": false, "default": "true"},
    {"key": "empty_check_delay_secs", "label": "Delai avant suppression salon vide (secondes)", "type": "number", "required": false, "default": "2"},

    {"key": "color_created", "label": "Couleur embed salon cree (hex sans #)", "type": "text", "required": false, "default": "2ecc71"},
    {"key": "color_deleted", "label": "Couleur embed salon supprime (hex sans #)", "type": "text", "required": false, "default": "e74c3c"},
    {"key": "color_joined", "label": "Couleur embed membre rejoint (hex sans #)", "type": "text", "required": false, "default": "3498db"},
    {"key": "color_left", "label": "Couleur embed membre parti (hex sans #)", "type": "text", "required": false, "default": "95a5a6"}
]' WHERE bot_name = 'voice-bot';
