-- Ticket-bot — « Probleme avec un moderateur » remonte desormais aux
-- PROPRIETAIRES du serveur (owner Discord) et non plus aux administrateurs.
-- Cette cle permet d'ajouter des co-fondateurs (2e owner...) : leurs IDs recoivent
-- l'acces au salon + le ping, en plus de l'owner Discord.

UPDATE bot_definitions SET config_schema = config_schema || '[
    {"key":"ticket_owner_ids","label":"Proprietaires (IDs, pour Probleme moderateur)","type":"text","required":false,"default":"","description":"IDs Discord des proprietaires/co-fondateurs (separes par des virgules) qui recoivent les tickets Probleme avec un moderateur. L owner du serveur est toujours inclus automatiquement."}
]'::jsonb
WHERE bot_name = 'ticket-bot'
  AND NOT (config_schema @> '[{"key":"ticket_owner_ids"}]'::jsonb);
