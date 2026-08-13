-- Migration 050 : Ajoute les cles de config pour les features avancees du progression-bot
-- (cooldown XP, streaks, multiplicateurs, recap, badges)

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "xp_cooldown_secs", "label": "Cooldown XP par message (secondes, 0=desactive)", "type": "number", "required": false, "default": "60"},
  {"key": "xp_channel_multipliers", "label": "Multiplicateurs XP par salon (channel_id:mult par ligne)", "type": "text", "required": false, "default": ""},
  {"key": "xp_role_multipliers", "label": "Multiplicateurs XP par role (role_id:mult par ligne)", "type": "text", "required": false, "default": ""},
  {"key": "weekly_recap_enabled", "label": "Recap hebdomadaire en DM", "type": "boolean", "required": false, "default": "false"},
  {"key": "streak_enabled", "label": "Systeme de streaks (bonus XP jours consecutifs)", "type": "boolean", "required": false, "default": "true"},
  {"key": "badges_enabled", "label": "Systeme de badges", "type": "boolean", "required": false, "default": "true"}
]'::jsonb
WHERE bot_name = 'progression-bot';
