-- Definition du game-bot dans bot_definitions
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'game-bot',
    'Game Bot',
    'Gestion des jeux mentionnables — les joueurs choisissent leurs jeux et se font ping quand quelqu''un mentionne le jeu.',
    '[
        {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
        {"key": "max_games", "label": "Nombre max de jeux par serveur (0 = illimite)", "type": "number", "required": false, "default": "50"},
        {"key": "role_color", "label": "Couleur des roles jeux (hex sans #)", "type": "text", "required": false, "default": "3498db"}
    ]'
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
