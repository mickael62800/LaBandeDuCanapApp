-- Nettoyage automatique : auto-purge de salons. Supprime periodiquement les
-- messages NON epingles des salons choisis. Tout se configure dans la page
-- Composants -> Nettoyage automatique (salons via une liste, + options).

UPDATE bot_definitions SET
    config_schema = config_schema || '[
        {"key": "autopurge_enabled", "type": "boolean", "label": "Auto-nettoyage de salons", "required": false, "default": "false", "description": "Supprime periodiquement les messages NON epingles des salons choisis."},
        {"key": "autopurge_channel_ids", "type": "channel_list", "label": "Salons a auto-nettoyer", "required": false, "depends_on": {"key": "autopurge_enabled", "equals": "true"}, "description": "Les salons dont les messages non epingles seront supprimes automatiquement."},
        {"key": "autopurge_interval_hours", "type": "number", "unit": "h", "min": 1, "max": 720, "label": "Frequence (heures)", "required": false, "default": "24", "depends_on": {"key": "autopurge_enabled", "equals": "true"}, "description": "Tous les combien nettoyer chaque salon. 1 = toutes les heures, 24 = chaque jour."},
        {"key": "autopurge_grace_hours", "type": "number", "unit": "h", "min": 0, "max": 168, "label": "Periode de grace (heures)", "required": false, "default": "0", "depends_on": {"key": "autopurge_enabled", "equals": "true"}, "description": "Ne pas supprimer les messages plus recents que ce delai. 0 = tout supprimer."},
        {"key": "autopurge_keep_bot", "type": "boolean", "label": "Garder les messages du bot", "required": false, "default": "false", "depends_on": {"key": "autopurge_enabled", "equals": "true"}, "description": "Ne pas supprimer les messages postes par le bot lui-meme."},
        {"key": "autopurge_log", "type": "boolean", "label": "Journaliser le nettoyage", "required": false, "default": "true", "depends_on": {"key": "autopurge_enabled", "equals": "true"}, "description": "Envoie un resume (X messages supprimes) dans les logs a chaque passage."}
    ]'::jsonb
WHERE bot_name = 'cleanup'
  AND NOT (config_schema @> '[{"key": "autopurge_enabled"}]'::jsonb);
