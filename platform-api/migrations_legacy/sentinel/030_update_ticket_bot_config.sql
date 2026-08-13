-- Ajout de parametres de personnalisation pour le ticket-bot
UPDATE bot_definitions SET config_schema = '[
    {"key": "assistance_channel_id", "label": "Salon d assistance", "type": "channel", "required": true},
    {"key": "ticket_category_id", "label": "Categorie Discord pour les tickets", "type": "channel", "required": false},
    {"key": "admin_role_id", "label": "Role Administrateur", "type": "role", "required": true},
    {"key": "moderator_role_id", "label": "Role Moderateur", "type": "role", "required": true},
    {"key": "max_open_per_user", "label": "Limite tickets ouverts par utilisateur (0 = illimite)", "type": "number", "required": false, "default": "0"},
    {"key": "inactive_close_days", "label": "Jours d inactivite avant fermeture auto (0 = desactive)", "type": "number", "required": false, "default": "7"},
    {"key": "close_delay_secs", "label": "Delai avant suppression du salon (secondes)", "type": "number", "required": false, "default": "5"},
    {"key": "transcript_dm_enabled", "label": "Envoyer le transcript en DM a la fermeture", "type": "boolean", "required": false, "default": "true"},
    {"key": "color_normal", "label": "Couleur embed ticket normal (hex sans #)", "type": "text", "required": false, "default": "2ecc71"},
    {"key": "color_urgent", "label": "Couleur embed ticket urgent (hex sans #)", "type": "text", "required": false, "default": "ff6600"},
    {"key": "color_confidential", "label": "Couleur embed ticket confidentiel (hex sans #)", "type": "text", "required": false, "default": "e74c3c"},
    {"key": "color_staff", "label": "Couleur embed commandes staff (hex sans #)", "type": "text", "required": false, "default": "e67e22"},
    {"key": "color_user", "label": "Couleur embed commandes utilisateur (hex sans #)", "type": "text", "required": false, "default": "3498db"},
    {"key": "welcome_message", "label": "Message d accueil personnalise (laisser vide = defaut)", "type": "text", "required": false, "default": ""}
]' WHERE bot_name = 'ticket-bot';
