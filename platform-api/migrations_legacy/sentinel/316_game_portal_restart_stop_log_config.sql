-- ============================================================================
-- Game Portal — exposition des reglages restart/stuck/stop/log codes en dur.
-- ============================================================================
-- Plusieurs comportements runtime du game-portal etaient codes en dur dans le
-- domaine / les services :
--   * backoff d'auto-restart : base 30s, plafond 3600s (30 * 2^n)
--   * seuil "stuck transition" du reconciler : 10 min
--   * grace docker stop : 30s
--   * cap de lignes de logs recuperees : 1000
-- On les ajoute au config_schema du module `game-portal` pour les rendre
-- editables par serveur (defaults = valeurs actuelles -> zero changement de
-- comportement). Le TTL de reservation de port reste global (infra) et se
-- regle via l'env `GAME_PORTAL_PORT_RESERVATION_TTL_SECS`.
--
-- Idempotent : n'ajoute chaque cle que si elle est absente du schema.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "restart_backoff_base_secs", "label": "Auto-restart : base du backoff (secondes)", "type": "number", "required": false, "default": "30", "min": 1, "max": 3600, "unit": "s", "description": "Delai de base du backoff exponentiel (base * 2^tentatives) entre deux redemarrages auto apres crash.", "depends_on": {"key": "auto_restart_on_crash", "equals": "true"}},
    {"key": "restart_backoff_cap_secs", "label": "Auto-restart : plafond du backoff (secondes)", "type": "number", "required": false, "default": "3600", "min": 1, "max": 86400, "unit": "s", "description": "Plafond du delai de backoff entre deux redemarrages auto (ne monte jamais au-dela, meme apres de nombreuses tentatives).", "depends_on": {"key": "auto_restart_on_crash", "equals": "true"}},
    {"key": "stuck_transition_threshold_minutes", "label": "Reconciler : seuil etat transitoire bloque (minutes)", "type": "number", "required": false, "default": "10", "min": 1, "max": 1440, "unit": "min", "description": "Au-dela de ce delai dans un etat Starting/Stopping sans progression, le reconciler force la resolution (Error ou etat reel du container)."},
    {"key": "stop_timeout_secs", "label": "Arret : delai de grace avant kill (secondes)", "type": "number", "required": false, "default": "30", "min": 1, "max": 600, "unit": "s", "description": "Delai laisse a un container pour s''arreter proprement (docker stop -t) avant kill force."},
    {"key": "max_log_lines", "label": "Logs : nombre max de lignes par requete", "type": "number", "required": false, "default": "1000", "min": 1, "max": 5000, "unit": "lignes", "description": "Borne dure du nombre de lignes de logs recuperables en une seule requete (protege la memoire et la taille du payload)."}
]'::jsonb
WHERE bot_name = 'game-portal'
  AND NOT (config_schema @> '[{"key": "restart_backoff_base_secs"}]'::jsonb);
