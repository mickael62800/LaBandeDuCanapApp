-- Ajouter le parametre combat_expire_secs dans le config_schema du coude-bot
UPDATE bot_definitions SET config_schema = config_schema || '[
  {"key":"combat_expire_secs","label":"Expiration defi (secondes)","type":"number","required":false,"default":"86400","description":"Duree avant qu un defi expire si le defenseur ne repond pas (defaut: 86400 = 24h). Le defenseur recoit une penalite de lachete."}
]'::jsonb
WHERE bot_name = 'coude-bot';
