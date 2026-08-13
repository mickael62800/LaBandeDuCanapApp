-- Salon de log des bans de verification d'age (welcome-bot).
-- Quand un membre saisit un age < age_minimum, il est banni temporairement ;
-- le bot poste desormais une card de log dans ce salon (cible, age declare,
-- minimum, duree du ban, deban auto). Vide = pas de log.
-- Idempotent : ajoute la cle seulement si absente.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "age_ban_log_channel_id", "label": "Salon de log des bans d age", "type": "channel", "required": false, "description": "Salon ou le bot poste une card quand un membre est banni par la verification d age (age saisi sous le minimum). Affiche l age declare, le minimum, la duree du ban et la date de deban auto. Vide = pas de log.", "depends_on": {"key": "age_check_enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'welcome-bot'
  AND NOT (config_schema @> '[{"key": "age_ban_log_channel_id"}]'::jsonb);
