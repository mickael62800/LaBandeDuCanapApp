-- Jeu « Influence » — role Discord par organisation.
-- Le fondateur (payant, en coins) ou un moderateur (gratuit) cree un role
-- Discord au nom de l'orga ; le fondateur le recoit, et les membres qui
-- rejoignent l'obtiennent aussi.

ALTER TABLE influence_organizations ADD COLUMN IF NOT EXISTS discord_role_id TEXT;

UPDATE bot_definitions SET config_schema = config_schema || '[
    {"key":"influence_org_role_cost","label":"Cout du role Discord d une organisation (coins)","type":"number","required":false,"default":"2000","description":"Coins preleves au fondateur qui cree le role Discord de son organisation (gratuit pour un moderateur)."}
]'::jsonb
WHERE bot_name = 'influence-bot'
  AND NOT (config_schema @> '[{"key":"influence_org_role_cost"}]'::jsonb);
