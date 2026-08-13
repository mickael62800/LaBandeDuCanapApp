-- Rend le timeout AFK blackjack parametrable par serveur.
-- La cle vit sous `blackjack-bot` (module principal), lue par le
-- blackjack-cleanup-worker via LEFT JOIN.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "afk_timeout_secs", "label": "Timeout AFK tables (secondes)", "type": "number", "required": false, "default": "600", "description": "Duree d inactivite avant fermeture automatique d une table blackjack. Default 600s = 10 min."}
]'::jsonb
WHERE bot_name = 'blackjack-bot'
  AND NOT (config_schema @> '[{"key": "afk_timeout_secs"}]'::jsonb);
