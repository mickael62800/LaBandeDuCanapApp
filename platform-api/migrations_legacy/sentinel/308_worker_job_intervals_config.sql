-- ============================================================================
-- Worker job intervals — exposition de 4 intervalles codes en dur.
-- ============================================================================
-- Quatre jobs `spawn_periodic` du worker passaient un intervalle litteral au
-- lieu de lire WorkerConfig, contournant le pattern config du crate. On les
-- rend reglables (const/env/DB) cote worker et editables au dashboard ici, en
-- miroir des autres intervals worker (game-portal migration 216/306, analytics
-- migration 225). Le worker lit la valeur via WorkerConfig.apply_db_config.
--
-- Placement des cles (bot_name doit etre dans WORKER_MODULES cote worker pour
-- que la surcharge DB soit reellement lue) :
--   - automod_close_votes_secs, automod_cleanup_cards_secs -> automod-bot
--     (automod-bot ajoute a WORKER_MODULES dans ce meme changement).
--   - monthly_ranking_check_secs -> analytics (module du job publish_monthly_
--     ranking, deja dans WORKER_MODULES, ou vivent les autres cadences worker).
--   - tournament_check_secs -> coude-bot (deja dans WORKER_MODULES).
--
-- Defauts = les litteraux actuels (aucun changement de comportement tant que
-- non reconfigure). Idempotent : chaque cle n'est ajoutee que si absente.

-- automod-bot : cloture des votes a echeance (CHEMIN CRITIQUE) + nettoyage cartes
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "automod_close_votes_secs", "label": "Worker : intervalle cloture des votes", "type": "number", "required": false, "default": "60", "min": 10, "max": 600, "unit": "s", "description": "Frequence a laquelle le worker ferme les cartes de vote de moderation arrivees a echeance. CRITIQUE : seule voie qui cloture les votes a leur deadline. Une valeur trop haute retarde la resolution des sanctions."}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key": "automod_close_votes_secs"}]'::jsonb);

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "automod_cleanup_cards_secs", "label": "Worker : intervalle nettoyage cartes closes", "type": "number", "required": false, "default": "86400", "min": 3600, "max": 604800, "unit": "s", "description": "Frequence de purge des cartes de moderation closes depuis plus d un mois. La review et le transcript restent en base (trace web conservee)."}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key": "automod_cleanup_cards_secs"}]'::jsonb);

-- analytics : cadence de check de publication du classement mensuel
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "monthly_ranking_check_secs", "label": "Worker : intervalle check classement mensuel", "type": "number", "required": false, "default": "3600", "min": 300, "max": 86400, "unit": "s", "description": "Frequence a laquelle le worker verifie s il faut publier le classement mensuel. L API ne publie qu au passage de mois, donc un tick horaire suffit."}
]'::jsonb
WHERE bot_name = 'analytics'
  AND NOT (config_schema @> '[{"key": "monthly_ranking_check_secs"}]'::jsonb);

-- coude-bot : cadence de check de resolution du tournoi hebdomadaire
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "tournament_check_secs", "label": "Worker : intervalle check tournoi", "type": "number", "required": false, "default": "3600", "min": 300, "max": 3600, "unit": "s", "description": "Frequence a laquelle le worker verifie s il faut resoudre le tournoi. Le job n agit que dans la fenetre dimanche >= 23h UTC ; un intervalle <= 1h garantit qu un tick tombe dans cette heure."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "tournament_check_secs"}]'::jsonb);
