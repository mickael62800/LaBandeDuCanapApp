-- Ajoute la cle "enabled" dans le config_schema de chaque bot et worker.
-- Permet d'afficher le toggle d'activation dans le formulaire de l'app desktop.
-- La logique de lecture de cette cle est deja implementee cote Rust (bots + workers).

UPDATE bot_definitions
SET config_schema = '[{"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"}]'::jsonb || config_schema
WHERE bot_name = 'automod-bot';

UPDATE bot_definitions
SET config_schema = '[{"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"}]'::jsonb || config_schema
WHERE bot_name = 'moderation-bot';

UPDATE bot_definitions
SET config_schema = '[{"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"}]'::jsonb || config_schema
WHERE bot_name = 'security-bot';

UPDATE bot_definitions
SET config_schema = '[{"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"}]'::jsonb || config_schema
WHERE bot_name = 'progression-bot';

UPDATE bot_definitions
SET config_schema = '[{"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"}]'::jsonb || config_schema
WHERE bot_name = 'ticket-bot';

UPDATE bot_definitions
SET config_schema = '[{"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"}]'::jsonb || config_schema
WHERE bot_name = 'voice-bot';

UPDATE bot_definitions
SET config_schema = '[{"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"}]'::jsonb || config_schema
WHERE bot_name = 'image-bot';

UPDATE bot_definitions
SET config_schema = '[{"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true"}]'::jsonb || config_schema
WHERE bot_name = 'moderation-worker';

UPDATE bot_definitions
SET config_schema = '[{"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true"}]'::jsonb || config_schema
WHERE bot_name = 'analytics-worker';
