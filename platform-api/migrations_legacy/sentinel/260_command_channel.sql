-- Salons "commandes uniquement" : tout message texte classique y est supprime
-- en silence (seul l'owner peut ecrire). Les commandes slash (interactions)
-- ne sont pas des messages -> elles continuent de fonctionner. Les messages
-- des bots ne sont jamais supprimes.

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'command-channel-bot',
    'Salons a commandes',
    'Salons ou seules les commandes sont autorisees : tout message classique est supprime (sauf owner et bots).',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "false", "description": "Active la suppression des messages classiques dans les salons designes."},
        {"key": "command_channels", "label": "Salons a commandes uniquement", "type": "text", "required": false, "default": "", "description": "Salons ou seules les commandes sont autorisees. Tout message classique y est supprime en silence (sauf owner et bots).", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
)
ON CONFLICT (bot_name) DO UPDATE
    SET display_name = EXCLUDED.display_name, description = EXCLUDED.description;
