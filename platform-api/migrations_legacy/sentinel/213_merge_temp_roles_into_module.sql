-- Phase composants — `temp-roles-worker` n'a pas de module bot associe
-- (la logique d'assignation cote bot est dans community-bot). On le
-- renomme simplement en `temp_roles` pour cohérence avec WORKER_MODULES,
-- et on enrichit la description.
--
-- Schema : 2 cles (enabled + scan_interval), avec depends_on cascade.

-- 1) Cree (ou met a jour) la definition unifiee.
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'temp_roles',
    'Roles temporaires',
    'Retire automatiquement les roles temporaires expires (assignes via /role temp ou par d''autres modules). Sans ce module, les roles temporaires ne sont jamais retires.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Si OFF, les roles temporaires expires ne seront plus retires automatiquement."},
        {"key": "temp_roles_scan_interval", "label": "Intervalle scan roles expires", "type": "number", "required": false, "default": "60", "min": 10, "max": 3600, "unit": "s", "description": "Frequence de scan des roles temporaires a retirer. 60s = bon compromis precision/charge.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;

-- 2) Supprime l'ancienne definition worker.
DELETE FROM bot_definitions WHERE bot_name = 'temp-roles-worker';
