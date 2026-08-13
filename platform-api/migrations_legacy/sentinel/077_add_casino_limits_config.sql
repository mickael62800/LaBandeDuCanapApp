-- Ajouter les parametres de limite casino dans le config_schema du coude-bot.
-- On reconstruit le schema complet pour inclure les nouveaux champs.

UPDATE bot_definitions SET config_schema = (
    SELECT config_schema || '[
        {"key":"casino_cooldown_secs","label":"Cooldown casino (secondes)","type":"number","required":false,"default":"300","description":"Temps d attente entre chaque partie de casino en secondes (defaut: 5 minutes)."},
        {"key":"casino_max_daily","label":"Max parties casino/jour","type":"number","required":false,"default":"10","description":"Nombre maximum de parties de casino par joueur par jour (0 = illimite)."},
        {"key":"casino_max_daily_gain","label":"Max gain casino/jour","type":"number","required":false,"default":"5000","description":"Plafond de gains au casino par joueur par jour en coins (0 = illimite)."}
    ]'::jsonb
    FROM bot_definitions WHERE bot_name = 'coude-bot'
)
WHERE bot_name = 'coude-bot';
