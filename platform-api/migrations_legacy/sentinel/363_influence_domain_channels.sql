-- Influence : salons de DOMAINE (un salon par entité du jeu) + restriction des
-- commandes à leur salon. Ajoute deux clés au schéma influence-bot (append jsonb,
-- idempotent).
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key":"influence_domain_channels_enabled","label":"Salons de domaine (par entité)","type":"boolean","required":false,"default":"false","description":"Crée un salon par entité du jeu (citoyen, lois-votes, renseignement, organisations, actualité) et restreint chaque commande à son salon."},
    {"key":"influence_domain_category_id","label":"Catégorie des salons de domaine","type":"category","required":false,"default":"","description":"Catégorie où ranger les salons de domaine. Vide = catégorie \"Influence\" créée par le bot.","depends_on":{"key":"influence_domain_channels_enabled","equals":"true"}}
]'::jsonb
WHERE bot_name = 'influence-bot'
  AND NOT (config_schema @> '[{"key":"influence_domain_channels_enabled"}]'::jsonb);
