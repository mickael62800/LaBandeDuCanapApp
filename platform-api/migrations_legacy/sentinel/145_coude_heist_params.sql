-- Expose le cooldown et la duree de prison du braquage dans le config_schema
-- de coude-bot (configurables par serveur via la web UI).

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "heist_cooldown_days", "label": "Cooldown braquage (jours)", "type": "number", "required": false, "default": "7", "description": "Delai minimum entre deux tentatives de braquage par joueur."},
  {"key": "heist_prison_hours", "label": "Duree de prison apres echec (heures)", "type": "number", "required": false, "default": "24", "description": "Temps pendant lequel un joueur reste en prison apres un braquage rate (aucune action de jeu possible)."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "heist_cooldown_days"}]'::jsonb);
