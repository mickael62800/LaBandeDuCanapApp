-- Notification privee uniforme pour toute sanction AutoMod (auto ou revue).
UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key":"sanction_notify_member","type":"boolean","label":"Informer le membre en message privé après toute sanction","default":"true","required":false,"depends_on":{"key":"enabled","equals":"true"},"description":"Envoie un DM best-effort après un warn, une suppression, un mute, un kick ou un ban AutoMod. Le message contient le motif, la durée éventuelle et le droit d appel si celui-ci est activé."}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key":"sanction_notify_member"}]'::jsonb);
