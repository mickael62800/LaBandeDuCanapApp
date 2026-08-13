-- Ajout de la limite quotidienne de vols dans les definitions coude-bot
UPDATE bot_definitions SET config_schema = config_schema::jsonb || '[
    {"key": "steal_max_daily", "label": "Vols max par jour (0 = illimite)", "type": "number", "required": false, "default": "5"}
]'::jsonb WHERE bot_name = 'coude-bot';
