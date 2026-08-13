-- Recompense VIP : un membre qui a fait au moins N bumps (cumul all-time)
-- recoit automatiquement un role VIP, en plus des coins par bump.
--
-- Idempotent : on n'ajoute les champs que s'ils ne sont pas deja presents.
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "vip_enabled", "label": "Role VIP apres X bumps", "type": "boolean", "required": false, "default": "false", "description": "Attribue un role VIP a partir d un certain nombre de bumps cumules."},
    {"key": "vip_role_id", "label": "Role VIP a attribuer", "type": "role", "required": false, "default": "", "description": "Role donne au membre une fois le seuil de bumps atteint."},
    {"key": "vip_bump_threshold", "label": "Nombre de bumps pour devenir VIP", "type": "number", "required": false, "default": "10", "description": "Total de bumps (cumul) requis pour debloquer le role VIP."}
]'::jsonb
WHERE bot_name = 'bump-bot'
  AND NOT (config_schema @> '[{"key": "vip_enabled"}]'::jsonb);
