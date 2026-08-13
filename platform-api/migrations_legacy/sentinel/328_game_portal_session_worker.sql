-- Game Portal — support worker des sessions : ping quotidien + revelation IP.

-- Dernier ping quotidien emis pour cette session (evite les doublons intra-jour).
ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS last_daily_ping_at TIMESTAMPTZ;

-- Hote/IP public communique aux joueurs a la revelation (le port vient de
-- host_port). Vide = on n'affiche que le port.
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "session_public_host", "label": "Hote public du serveur (IP ou domaine, affiche a la revelation)", "type": "text", "required": false}
]'::jsonb
WHERE bot_name = 'game-portal'
  AND NOT (config_schema @> '[{"key": "session_public_host"}]'::jsonb);
