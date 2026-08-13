-- Coude : salons de DOMAINE (un salon par entité du jeu) + restriction des
-- commandes à leur salon. Ajoute deux clés au schéma coude-bot (append jsonb,
-- idempotent).
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key":"coude_domain_channels_enabled","label":"Salons de domaine (par entité)","type":"boolean","required":false,"default":"false","description":"Crée un salon par entité du jeu (combat, personnage, économie, fun) et restreint chaque commande à son salon."},
    {"key":"coude_domain_category_id","label":"Catégorie des salons de domaine","type":"category","required":false,"default":"","description":"Catégorie où ranger les salons de domaine. Vide = catégorie \"Coude\" créée par le bot.","depends_on":{"key":"coude_domain_channels_enabled","equals":"true"}}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key":"coude_domain_channels_enabled"}]'::jsonb);
