-- Tamagotchi M2a — parametres Entrainer + prix de la boutique.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "train_stat_gain", "label": "Gain de stat (Entrainer)", "type": "number", "required": false, "default": "1", "min": 1, "max": 20, "description": "Points ajoutes a la stat entrainee (FORCE/VITALITE/AGILITE).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "train_cost", "label": "Cout Entrainer", "type": "number", "required": false, "default": "0", "min": 0, "max": 1000000, "unit": "coins", "description": "Cout en coins d une seance d entrainement (0 = gratuit, seule l energie est consommee).", "depends_on": {"key": "enabled", "equals": "true"}},

    {"key": "shop_croquettes_price", "label": "Prix Croquettes (+faim)", "type": "number", "required": false, "default": "15", "min": 0, "max": 1000000, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_repas_price", "label": "Prix Repas premium (+faim++)", "type": "number", "required": false, "default": "40", "min": 0, "max": 1000000, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_boisson_price", "label": "Prix Boisson energisante (+energie)", "type": "number", "required": false, "default": "25", "min": 0, "max": 1000000, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_jouet_price", "label": "Prix Jouet (+bonheur)", "type": "number", "required": false, "default": "20", "min": 0, "max": 1000000, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_potion_price", "label": "Prix Potion de soin (guerit)", "type": "number", "required": false, "default": "100", "min": 0, "max": 1000000, "unit": "coins", "description": "Potion qui guerit la maladie et restaure un peu toutes les jauges.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'tamagotchi-bot'
  AND NOT (config_schema @> '[{"key": "train_stat_gain"}]'::jsonb);
