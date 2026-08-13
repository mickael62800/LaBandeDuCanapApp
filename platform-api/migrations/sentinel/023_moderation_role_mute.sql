-- Mute par role : permet de laisser au membre l'acces a un salon d'appel
-- lorsque ses messages prives Discord sont fermes. L'expiration est geree par
-- la table existante temp_roles et son worker.
UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key":"mute_uses_role","type":"boolean","label":"Mute via un role dedie","default":"false","required":false,"depends_on":{"key":"enabled","equals":"true"},"description":"Utilise le role ci-dessous au lieu du timeout Discord. Configure ce role pour bloquer les salons normaux et autorise-le dans le salon d appel."},
  {"key":"mute_role_id","type":"role","label":"Role de mute","default":"","required":false,"depends_on":{"key":"mute_uses_role","equals":"true"},"description":"Role ajoute temporairement au membre mute puis retire automatiquement a l expiration. Le role doit etre sous celui de SentinelBot."}
]'::jsonb
WHERE bot_name = 'moderation-bot'
  AND NOT (config_schema @> '[{"key":"mute_uses_role"}]'::jsonb);
