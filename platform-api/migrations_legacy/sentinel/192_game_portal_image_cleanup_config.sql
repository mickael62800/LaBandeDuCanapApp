-- ============================================================================
-- Game Portal — config image-cleanup (auto-remove images Docker non utilisees)
-- ============================================================================
-- Etend le schema bot_definitions:game-portal pour exposer 2 nouvelles cles
-- editables depuis la page Composants :
--  - auto_remove_unused_images : active/desactive le job de cleanup
--  - unused_image_grace_days   : nb de jours de grace avant suppression
--
-- Le job tourne 1x par jour dans game-portal-worker. Pour chaque template
-- du catalogue : si aucun serveur actif n'utilise l'image ET la derniere
-- activite est plus ancienne que la grace period, l'image est supprimee.
-- Si un container utilise encore l'image, Docker refuse la suppression
-- (defense en profondeur, le job log warn et passe).

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(entry) FROM (
        SELECT entry FROM jsonb_array_elements(config_schema::jsonb) AS entry
        UNION ALL
        SELECT '{
            "key": "auto_remove_unused_images",
            "label": "Suppression auto images Docker non utilisees",
            "type": "boolean",
            "required": false,
            "default": "true",
            "description": "Active le job game-portal-worker qui supprime les images Docker des templates non utilises depuis N jours. Liberation de disque (Minecraft = 500 MB, Palworld = 8 GB)."
        }'::jsonb
        WHERE NOT EXISTS (
            SELECT 1 FROM jsonb_array_elements(config_schema::jsonb) e
            WHERE e->>'key' = 'auto_remove_unused_images'
        )
        UNION ALL
        SELECT '{
            "key": "unused_image_grace_days",
            "label": "Jours de grace avant suppression image",
            "type": "number",
            "required": false,
            "default": "7",
            "description": "Nb de jours sans aucun serveur actif utilisant le template avant que son image Docker soit supprimee. 0 = desactive. Si tu relances un serveur apres suppression, l image se re-pull automatiquement (Minecraft : 1-2 min)."
        }'::jsonb
        WHERE NOT EXISTS (
            SELECT 1 FROM jsonb_array_elements(config_schema::jsonb) e
            WHERE e->>'key' = 'unused_image_grace_days'
        )
    ) merged
)
WHERE bot_name = 'game-portal';
