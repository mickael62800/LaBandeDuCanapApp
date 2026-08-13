-- ============================================================================
-- Worker : exposition des valeurs MED codees en dur (batch IA, publication
-- annonces, delai daily_chaos, garde-fous export).
-- ============================================================================
-- En miroir de la migration 308 (intervalles worker). Chaque valeur est
-- desormais reglable const/env/DB cote worker (WorkerConfig) et editable au
-- dashboard ici. Le worker lit la surcharge via WorkerConfig.apply_db_config.
--
-- Placement des cles (bot_name doit etre dans WORKER_MODULES cote worker pour
-- que la surcharge DB soit reellement lue — tous verifies presents) :
--   - ai_batch_size                     -> ai
--   - announcement_publish_interval_secs -> announcements
--   - daily_chaos_min_delay_secs / daily_chaos_max_delay_secs -> coude-bot
--   - max_rows_per_export / export_processing_timeout_secs    -> export
--
-- Defauts = les litteraux actuels (aucun changement de comportement tant que
-- non reconfigure). Idempotent : chaque cle n'est ajoutee que si absente.

-- ai : taille du batch de jobs IA claimes par tick
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "ai_batch_size", "label": "Worker : taille du batch de jobs IA", "type": "number", "required": false, "default": "5", "min": 1, "max": 100, "unit": "jobs", "description": "Nombre de jobs IA claimes et traites a chaque tick de depilage. Plus haut = meilleur debit mais plus de charge sur l API d inference."}
]'::jsonb
WHERE bot_name = 'ai'
  AND NOT (config_schema @> '[{"key": "ai_batch_size"}]'::jsonb);

-- announcements : cadence de publication des annonces dues
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "announcement_publish_interval_secs", "label": "Worker : intervalle publication annonces", "type": "number", "required": false, "default": "3600", "min": 60, "max": 86400, "unit": "s", "description": "Frequence a laquelle le worker publie les annonces dues sur la stream Redis. La boucle s aligne sur l heure pile au demarrage ; garder 3600 preserve l alignement HH:00."}
]'::jsonb
WHERE bot_name = 'announcements'
  AND NOT (config_schema @> '[{"key": "announcement_publish_interval_secs"}]'::jsonb);

-- coude-bot : bornes du delai aleatoire de la Roue du Destin (daily_chaos)
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "daily_chaos_min_delay_secs", "label": "Worker : delai min Roue du Destin", "type": "number", "required": false, "default": "7200", "min": 300, "max": 86400, "unit": "s", "description": "Borne basse du delai aleatoire entre deux declenchements de la Roue du Destin. Doit rester <= au delai max."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "daily_chaos_min_delay_secs"}]'::jsonb);

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "daily_chaos_max_delay_secs", "label": "Worker : delai max Roue du Destin", "type": "number", "required": false, "default": "21600", "min": 300, "max": 172800, "unit": "s", "description": "Borne haute du delai aleatoire entre deux declenchements de la Roue du Destin. Si < au delai min, le worker le ramene au delai min."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "daily_chaos_max_delay_secs"}]'::jsonb);

-- export : garde-fous memoire et timeout zombie
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "max_rows_per_export", "label": "Worker : max lignes par export", "type": "number", "required": false, "default": "50000", "min": 1, "max": 10000000, "unit": "lignes", "description": "Garde-fou memoire : nombre max de lignes retournees par un export. Au-dela l API tronque. 50k lignes JSON ~ 20-50 MB."}
]'::jsonb
WHERE bot_name = 'export'
  AND NOT (config_schema @> '[{"key": "max_rows_per_export"}]'::jsonb);

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "export_processing_timeout_secs", "label": "Worker : timeout job export zombie", "type": "number", "required": false, "default": "300", "min": 30, "max": 86400, "unit": "s", "description": "Duree au-dela de laquelle un export bloque en processing est considere zombie (worker crash) et remis en pending pour retry."}
]'::jsonb
WHERE bot_name = 'export'
  AND NOT (config_schema @> '[{"key": "export_processing_timeout_secs"}]'::jsonb);
