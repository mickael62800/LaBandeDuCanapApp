-- Ajouter le parametre default_role_id dans le config_schema du progression-bot
UPDATE bot_definitions SET config_schema = config_schema || '[
  {"key":"default_role_id","label":"Role par defaut (nouvel arrivant)","type":"role","required":false,"default":"","description":"ID du role attribue automatiquement a chaque nouveau membre qui rejoint le serveur. Laissez vide pour desactiver."}
]'::jsonb
WHERE bot_name = 'progression-bot';
