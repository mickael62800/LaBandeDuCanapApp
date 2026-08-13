-- Renommage roles-bot → community-bot dans les tables de configuration

UPDATE bot_definitions
SET bot_name = 'community-bot',
    display_name = 'Community',
    description = 'Auto-roles, panels de roles, onboarding, parcours communautaire'
WHERE bot_name = 'roles-bot';

UPDATE bot_guild_config
SET bot_name = 'community-bot'
WHERE bot_name = 'roles-bot';

UPDATE logs
SET bot = 'community-bot'
WHERE bot = 'roles-bot';
