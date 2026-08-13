-- Composant "Panneau d'aide" (help-bot) : le bot publie et maintient tout seul,
-- dans un salon, un catalogue de TOUTES les commandes disponibles, trié par
-- catégorie, avec leur description. Auto-généré depuis le registre de commandes
-- (aucun copier-coller), auto-mis à jour au démarrage du bot (idempotent : il
-- remplace son ancien message au lieu d'en créer un nouveau).
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'help-bot',
    'Panneau d''aide',
    'Publie automatiquement dans un salon un catalogue de toutes les commandes du serveur (triées par categorie, avec description). Genere et mis a jour par le bot, sans intervention manuelle.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Publie/maintient le panneau d aide."},
        {"key": "channel_id", "label": "Salon du panneau", "type": "text", "required": false, "default": "", "description": "ID du salon ou publier le catalogue. Vide = le bot cree/utilise un salon #commandes.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    config_schema = EXCLUDED.config_schema,
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description;
