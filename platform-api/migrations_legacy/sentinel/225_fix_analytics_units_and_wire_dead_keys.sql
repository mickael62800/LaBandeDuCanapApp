-- Analytics : aligne le schema UI sur ce que le code interprete reellement,
-- conserve les 5 cles autrefois "dead" (track_voice_stats,
-- track_message_stats, data_retention_days, top_users_count, export_format)
-- car elles sont desormais cablees dans le code, et ajoute 3 cles pour la
-- publication automatique du Top users sur Discord.
--
-- Bugs corriges :
--   - daily_snapshot_interval : le worker fait `value * 3600`
--     (interprete en HEURES). Le schema 215 disait `default 86400 unit s
--     min 3600` → un admin saisissant 86400 declenchait un snapshot tous
--     les 9,86 ans.
--   - hourly_snapshot_interval : idem, le worker fait `value * 60`
--     (MINUTES). Le schema disait `default 3600 unit s min 600`.
--
-- Conversion des valeurs existantes :
--   - daily_snapshot_interval : si valeur >= 3600 (vraisemblablement saisie
--     en secondes), divise par 3600 (heures), sinon laisse intacte.
--   - hourly_snapshot_interval : si valeur >= 600, divise par 60 (minutes).
--
-- Nouvelles cles publication Top users :
--   - top_users_publish_enabled (bool) : active la publication auto
--   - top_users_publish_channel_id (channel) : salon de destination
--   - top_users_publish_interval_days (number, default 7) : frequence

UPDATE bot_definitions SET
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Si OFF : pas de snapshots, les graphiques du dashboard restent figes."},

        {"key": "track_voice_stats", "label": "Tracker stats vocales", "type": "boolean", "required": false, "default": "true", "description": "Inclut voice_minutes dans les snapshots quotidiens. Si OFF, la colonne reste a 0.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "track_message_stats", "label": "Tracker stats messages", "type": "boolean", "required": false, "default": "true", "description": "Inclut messages dans les snapshots quotidiens et horaires. Si OFF, les colonnes restent a 0.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "hourly_snapshot_interval", "label": "Intervalle snapshot horaire (minutes)", "type": "number", "required": false, "default": "60", "min": 10, "max": 1440, "unit": "min", "description": "Frequence des snapshots horaires. Valeur en MINUTES.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "daily_snapshot_interval", "label": "Intervalle snapshot journalier (heures)", "type": "number", "required": false, "default": "1", "min": 1, "max": 168, "unit": "h", "description": "Frequence des snapshots journaliers. Valeur en HEURES.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "data_retention_days", "label": "Retention des donnees", "type": "number", "required": false, "default": "90", "min": 0, "max": 3650, "unit": "j", "description": "Apres combien de jours les snapshots (daily_activity, hourly_activity) sont supprimes. 0 = illimite. Le job de cleanup tourne 1x/jour.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "top_users_count", "label": "Top utilisateurs (taille par defaut)", "type": "number", "required": false, "default": "10", "min": 1, "max": 100, "description": "Taille par defaut du top dans /api/analytics/top-infractors et dans le post Discord automatique.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "export_format", "label": "Format d''export par defaut", "type": "enum", "required": false, "default": "json", "options": [{"value": "json", "label": "JSON"}, {"value": "csv", "label": "CSV"}], "description": "Format par defaut de /api/analytics/export quand le client ne specifie pas ?format=. JSON pour API, CSV pour tableur.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "top_users_publish_enabled", "label": "Publier le Top users sur Discord", "type": "boolean", "required": false, "default": "false", "description": "Active la publication automatique du Top users dans un salon Discord.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "top_users_publish_channel_id", "label": "Salon de publication Top users", "type": "channel", "required": false, "description": "Salon ou poster l embed Top users.", "depends_on": {"key": "top_users_publish_enabled", "equals": "true"}},
        {"key": "top_users_publish_interval_days", "label": "Frequence publication Top users (jours)", "type": "number", "required": false, "default": "7", "min": 1, "max": 90, "unit": "j", "description": "Intervalle minimal entre deux publications. Le worker tick chaque heure et publie quand l interval est ecoule.", "depends_on": {"key": "top_users_publish_enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'analytics';

-- Conversion seconds → hours pour daily.
UPDATE bot_guild_config
   SET config_value = (config_value::int / 3600)::text
 WHERE bot_name = 'analytics'
   AND config_key = 'daily_snapshot_interval'
   AND config_value ~ '^\d+$'
   AND config_value::int >= 3600;

-- Conversion seconds → minutes pour hourly.
UPDATE bot_guild_config
   SET config_value = (config_value::int / 60)::text
 WHERE bot_name = 'analytics'
   AND config_key = 'hourly_snapshot_interval'
   AND config_value ~ '^\d+$'
   AND config_value::int >= 600;
