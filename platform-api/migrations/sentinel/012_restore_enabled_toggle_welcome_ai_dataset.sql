-- Restaure l'interrupteur maitre `enabled` sur welcome-bot et ai-dataset-bot.
--
-- Lors de la consolidation du schema (001_init), ces deux modules ont perdu
-- leur cle `enabled` de tete. Consequence : aucun toggle « Module actif » dans
-- la page Composants -> impossible d'activer le module -> pour welcome-bot, la
-- tuile « Bienvenue » (gardee par requiredBot: welcome-bot) n'apparaissait
-- jamais. On la re-prepend si elle manque.

UPDATE bot_definitions SET
    config_schema = jsonb_build_array(
        jsonb_build_object(
            'key', 'enabled',
            'label', 'Module actif',
            'type', 'boolean',
            'required', false,
            'default', 'true',
            'description', 'Interrupteur principal : si OFF, le module est entierement desactive (aucune action, aucune commande).'
        )
    ) || config_schema
WHERE bot_name IN ('welcome-bot', 'ai-dataset-bot')
  AND NOT (config_schema @> '[{"key": "enabled"}]'::jsonb);
