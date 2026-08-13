-- Ajouter les parametres manquants dans le config_schema du coude-bot
UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "cancel_penalty", "label": "Penalite annulation (%)", "type": "number", "required": false, "default": "5"},
  {"key": "bet_delay_secs", "label": "Delai paris apres acceptation (secondes)", "type": "number", "required": false, "default": "300"},
  {"key": "channel_combats", "label": "Salon des combats (ID)", "type": "channel", "required": false, "default": ""},
  {"key": "channel_leaderboard", "label": "Salon du leaderboard (ID)", "type": "channel", "required": false, "default": ""},
  {"key": "channel_profil", "label": "Salon profil (ID)", "type": "channel", "required": false, "default": ""},
  {"key": "channel_activites", "label": "Salon activites (ID)", "type": "channel", "required": false, "default": ""},
  {"key": "channel_announcements", "label": "Salon annonces (ID)", "type": "channel", "required": false, "default": ""},
  {"key": "channel_notifications", "label": "Salon notifications (ID)", "type": "channel", "required": false, "default": ""}
]'::jsonb
WHERE bot_name = 'coude-bot';
