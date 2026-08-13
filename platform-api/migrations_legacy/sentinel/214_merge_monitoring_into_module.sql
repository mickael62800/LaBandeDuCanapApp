-- Phase composants — `monitoring-worker` n'a pas de module bot
-- associe. On renomme en `monitoring` (cohérent avec WORKER_MODULES,
-- mig 204 a deja renomme les rows) et on enrichit la description.

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'monitoring',
    'Surveillance bots / workers',
    'Detecte les bots et workers offline via leurs heartbeats Redis et publie les events de transition (online/offline) consommes par la page Securite et les alertes Discord.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Si OFF : pas de detection offline, pas d alertes en cas de bot crash."},
        {"key": "check_interval", "label": "Intervalle de verification", "type": "number", "required": false, "default": "30", "min": 5, "max": 600, "unit": "s", "description": "Frequence du check des heartbeats. 30s = bon compromis reactivite/charge.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;

DELETE FROM bot_definitions WHERE bot_name = 'monitoring-worker';
