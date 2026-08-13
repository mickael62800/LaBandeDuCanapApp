-- Classement mensuel : exclusion de roles.
--
-- Permet d'exclure du classement les membres portant certains roles
-- (ex. staff, bots, bots musique). Liste d'IDs de roles (CSV), editee via
-- le dashboard (rendu multi-select de roles cote web).
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "monthly_ranking_excluded_roles", "label": "Roles exclus du classement mensuel", "type": "text", "required": false}
]'::jsonb
WHERE bot_name = 'progression-bot'
  AND NOT (config_schema @> '[{"key": "monthly_ranking_excluded_roles"}]'::jsonb);
