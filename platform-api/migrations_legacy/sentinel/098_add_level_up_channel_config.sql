-- Ajouter level_up_channel_id dans le config_schema du progression-bot
-- pour qu'il soit configurable dans la page Composants de l'app bureau.
UPDATE bot_definitions SET config_schema = config_schema::jsonb || '[
    {"key": "level_up_channel_id", "label": "Salon annonces level-up", "type": "channel", "required": false}
]'::jsonb
WHERE bot_name = 'progression-bot';
