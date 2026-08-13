-- Jeu « Influence » — salon Discord prive par organisation.
-- A la creation d'une organisation, le bot cree automatiquement un salon texte
-- prive (visible des seuls membres) ou l'equipe se coordonne. Les membres qui
-- rejoignent l'organisation y gagnent l'acces automatiquement.
-- La categorie d'accueil est configurable ; a defaut le bot cree/trouve une
-- categorie « Organisations ».

ALTER TABLE influence_organizations ADD COLUMN IF NOT EXISTS discord_channel_id TEXT;

UPDATE bot_definitions SET config_schema = config_schema || '[
    {"key":"influence_org_category_id","label":"Categorie des salons d organisations","type":"channel","required":false,"description":"Categorie Discord ou ranger les salons prives auto-crees des organisations. Vide = le bot cree/trouve une categorie Organisations."}
]'::jsonb
WHERE bot_name = 'influence-bot'
  AND NOT (config_schema @> '[{"key":"influence_org_category_id"}]'::jsonb);
