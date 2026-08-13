-- ============================================================================
-- Tamagotchi — effets (gains de jauges) de la boutique reglables par serveur.
-- ============================================================================
-- Les PRIX des objets etaient deja configurables (mig 257), mais les EFFETS
-- (combien de faim/bonheur/energie chaque objet restaure) restaient codes en
-- dur dans `panel.rs`, empechant tout equilibrage de l economie.
--
-- On expose chaque effet via une cle `shop_<objet>_<jauge>_gain` de la config
-- `tamagotchi-bot`, en miroir du nommage des prix (`shop_<objet>_price`).
--
-- Comportement : chaque cle retombe sur la valeur historique si absente/malformee
-- -> AUCUN changement de comportement tant que non reconfigure. Les jauges vont
-- de 0 a 100, donc gain >= 0 (min 0) et clampe a 100 a la lecture cote bot ;
-- la jauge resultante reste clampee a son max existant.
--
-- Idempotent : cles ajoutees seulement si absentes du schema.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "shop_croquettes_hunger_gain", "label": "Boutique — effet faim Croquettes", "type": "number", "required": false, "default": "25", "min": 0, "max": 100, "description": "Points de faim restaures par les Croquettes.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_repas_hunger_gain", "label": "Boutique — effet faim Repas premium", "type": "number", "required": false, "default": "60", "min": 0, "max": 100, "description": "Points de faim restaures par le Repas premium.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_boisson_energy_gain", "label": "Boutique — effet energie Boisson", "type": "number", "required": false, "default": "40", "min": 0, "max": 100, "description": "Points d energie restaures par la Boisson energisante.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_jouet_happiness_gain", "label": "Boutique — effet bonheur Jouet", "type": "number", "required": false, "default": "35", "min": 0, "max": 100, "description": "Points de bonheur restaures par le Jouet.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_potion_hunger_gain", "label": "Boutique — effet faim Potion de soin", "type": "number", "required": false, "default": "10", "min": 0, "max": 100, "description": "Points de faim restaures par la Potion de soin.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_potion_happiness_gain", "label": "Boutique — effet bonheur Potion de soin", "type": "number", "required": false, "default": "10", "min": 0, "max": 100, "description": "Points de bonheur restaures par la Potion de soin.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_potion_energy_gain", "label": "Boutique — effet energie Potion de soin", "type": "number", "required": false, "default": "10", "min": 0, "max": 100, "description": "Points d energie restaures par la Potion de soin.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'tamagotchi-bot'
  AND NOT (config_schema @> '[{"key": "shop_croquettes_hunger_gain"}]'::jsonb);
