-- Moderation-bot — annulation d'une sanction depuis le salon d'appel : elle
-- requiert desormais un VOTE de plusieurs moderateurs puis une validation par un
-- administrateur. Ce reglage fixe le quorum de votes modo requis.

UPDATE bot_definitions SET config_schema = config_schema || '[
    {"key":"appeal_cancel_quorum","label":"Votes modo requis pour annuler une sanction","type":"number","required":false,"default":"2","description":"Nombre de moderateurs distincts qui doivent voter avant qu un administrateur puisse valider l annulation d une sanction."}
]'::jsonb
WHERE bot_name = 'moderation-bot'
  AND NOT (config_schema @> '[{"key":"appeal_cancel_quorum"}]'::jsonb);
