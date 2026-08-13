-- ============================================================================
-- Game Portal — ajout de la cle `image_cleanup_interval_secs` au schema.
-- ============================================================================
-- Le worker declenche le job `image-cleanup` sur un timer (86400s = 24h par
-- defaut) mais aucune cle de config ne permettait de le regler : la valeur
-- etait codee en dur. On ajoute la cle manquante au config_schema du module
-- `game-portal`, en miroir des autres intervals worker (health_check,
-- idle_shutdown, reconciler). Le worker la lit via WorkerConfig
-- (game_image_cleanup_interval_secs).
--
-- Idempotent : n'ajoute la cle que si elle est absente du schema.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "image_cleanup_interval_secs", "label": "Worker : intervalle nettoyage images", "type": "number", "required": false, "default": "86400", "min": 3600, "max": 604800, "unit": "s", "description": "Frequence de scan des images Docker de templates non utilises a supprimer (liberation disque).", "depends_on": {"key": "auto_remove_unused_images", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'game-portal'
  AND NOT (config_schema @> '[{"key": "image_cleanup_interval_secs"}]'::jsonb);
