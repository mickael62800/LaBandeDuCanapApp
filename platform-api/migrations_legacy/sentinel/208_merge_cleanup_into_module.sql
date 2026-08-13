-- Phase composants — Fusion `cleanup-bot` (module) + `cleanup-worker`
-- (worker) dans une seule entree `cleanup`.
--
-- cleanup-bot ne contenait qu'une cle `enabled` (commandes /purge et
-- /cleanup cote Discord). cleanup-worker a 7 cles (retention DB +
-- VACUUM). On les fusionne avec hierarchie depends_on :
--
--   enabled (toggle principal)
--   ├─ voice_sessions_retention_days
--   ├─ logs_retention_days
--   ├─ closed_tickets_retention_days
--   ├─ cleanup_interval_hours
--   └─ vacuum_enabled (sub-toggle)
--         └─ vacuum_interval_hours
--
-- Le worker (`is_worker_enabled(pool, gid, "cleanup")`) lit deja les
-- configs sous bot_name='cleanup' (mig 204), donc rien a changer cote
-- code Rust.

-- 1) Migre les configs cleanup-bot (s'il y en a) vers le bot_name unifie.
DELETE FROM bot_guild_config wkr
    WHERE wkr.bot_name = 'cleanup-bot'
      AND EXISTS (
          SELECT 1 FROM bot_guild_config m
           WHERE m.bot_name = 'cleanup'
             AND m.guild_id = wkr.guild_id
             AND m.config_key = wkr.config_key
      );
UPDATE bot_guild_config SET bot_name = 'cleanup'
    WHERE bot_name = 'cleanup-bot';

-- 2) Cree/Update la nouvelle definition unifiee.
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'cleanup',
    'Nettoyage automatique',
    'Suppression periodique des donnees historiques (sessions vocales, logs, tickets fermes) + VACUUM Postgres pour optimiser la taille des tables. Les commandes /purge et /cleanup cote Discord sont aussi gerees ici.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active la suppression automatique + les commandes /purge et /cleanup."},
        {"key": "voice_sessions_retention_days", "label": "Retention sessions vocales", "type": "number", "required": false, "default": "90", "min": 7, "max": 365, "unit": "jours", "description": "Sessions plus anciennes que ce delai sont supprimees.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "logs_retention_days", "label": "Retention logs", "type": "number", "required": false, "default": "30", "min": 7, "max": 365, "unit": "jours", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "closed_tickets_retention_days", "label": "Retention tickets fermes", "type": "number", "required": false, "default": "180", "min": 7, "max": 365, "unit": "jours", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "cleanup_interval_hours", "label": "Intervalle nettoyage", "type": "number", "required": false, "default": "1", "min": 1, "max": 168, "unit": "h", "description": "Frequence du job de nettoyage. 1h = scan toutes les heures.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "vacuum_enabled", "label": "VACUUM automatique", "type": "boolean", "required": false, "default": "true", "description": "Reclamation de l espace disque + maintenance des index Postgres.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "vacuum_interval_hours", "label": "Intervalle VACUUM", "type": "number", "required": false, "default": "24", "min": 1, "max": 168, "unit": "h", "description": "Frequence du VACUUM ANALYZE. 24h est generalement un bon compromis.", "depends_on": {"key": "vacuum_enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;

-- 3) Supprime les anciennes definitions.
DELETE FROM bot_definitions WHERE bot_name IN ('cleanup-bot', 'cleanup-worker');
