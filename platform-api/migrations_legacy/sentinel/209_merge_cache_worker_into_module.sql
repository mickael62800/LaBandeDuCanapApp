-- Phase composants — Fusion `cache-worker` dans le module `cache`.
--
-- Pas de cache-bot existant : c'est purement de l'infra Redis (pre-
-- calcul analytics/dashboard/leaderboards). On cree directement
-- bot_name='cache' qui regroupe toutes les cles.
--
-- Le worker lit deja les configs sous bot_name='cache' via WORKER_MODULES.

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'cache',
    'Cache Redis (warm)',
    'Pre-calcul des donnees analytics, dashboard, leaderboards et stats vocales dans Redis pour des reponses instantanees cote frontend. Sans ce module, les requetes lourdes vont directement en DB a chaque vue.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Si OFF, le frontend tape la DB en direct (lent sur grosses guilds)."},
        {"key": "analytics_cache_refresh", "label": "Refresh cache analytics", "type": "number", "required": false, "default": "300", "min": 30, "max": 3600, "unit": "s", "description": "Frequence de regeneration du cache analytics. 300s = 5 min.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "dashboard_cache_refresh", "label": "Refresh cache dashboard", "type": "number", "required": false, "default": "600", "min": 30, "max": 3600, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "voice_stats_cache_refresh", "label": "Refresh stats vocales", "type": "number", "required": false, "default": "3600", "min": 60, "max": 86400, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "leaderboards_refresh", "label": "Refresh leaderboards", "type": "number", "required": false, "default": "300", "min": 30, "max": 3600, "unit": "s", "description": "Frequence de regeneration du cache leaderboards (top XP, top voice, etc.).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "user_cache_sync", "label": "Sync cache utilisateurs", "type": "number", "required": false, "default": "600", "min": 60, "max": 3600, "unit": "s", "description": "Frequence de synchronisation du cache des usernames Discord.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "partition_manager", "label": "Gestion partitions Postgres", "type": "number", "required": false, "default": "3600", "min": 600, "max": 86400, "unit": "s", "description": "Frequence de maintenance des partitions Postgres (creation/drop des tranches mensuelles).", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;

DELETE FROM bot_definitions WHERE bot_name = 'cache-worker';
