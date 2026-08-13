-- Renommage stats-bot → progression-bot dans les tables de configuration

UPDATE bot_definitions
SET bot_name = 'progression-bot',
    display_name = 'Progression',
    description = 'Suivi des messages, temps vocal, XP, niveaux et progression'
WHERE bot_name = 'stats-bot';

UPDATE bot_guild_config
SET bot_name = 'progression-bot'
WHERE bot_name = 'stats-bot';

UPDATE logs
SET bot = 'progression-bot'
WHERE bot = 'stats-bot';
