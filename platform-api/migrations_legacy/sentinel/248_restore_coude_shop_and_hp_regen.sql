-- coude-bot — restauration des prix de boutique + paliers de regen HP.
--
-- Regression : la mig 207 (fusion worker coude -> module) a reecrit
-- entierement le config_schema (SET config_schema = '[...]') et a perdu :
--   - 4 cles hp_regen_rate_* (ajoutees mig 119)
--   - 14 cles shop_*_price (ajoutees mig 131)
-- Ces cles sont LUES par le code (sentinel-bot/.../coude/guild_config.rs :
-- shop_price() et hp_regen_rate_*()), avec fallback sur des defauts hardcodes.
-- Absentes du schema -> impossible de regler les prix du shop ni la regen
-- depuis la page Composants. Ni 224 ni 234 ne les ont restaurees.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "hp_regen_rate_0_25", "label": "HP/h palier 0-25%", "type": "number", "required": false, "default": "100", "description": "Points de vie regeneres par heure quand le joueur est entre 0 et 25% de ses PV.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "hp_regen_rate_25_50", "label": "HP/h palier 25-50%", "type": "number", "required": false, "default": "50", "description": "Points de vie regeneres par heure entre 25 et 50% des PV.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "hp_regen_rate_50_75", "label": "HP/h palier 50-75%", "type": "number", "required": false, "default": "30", "description": "Points de vie regeneres par heure entre 50 et 75% des PV.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "hp_regen_rate_75_100", "label": "HP/h palier 75-100%", "type": "number", "required": false, "default": "10", "description": "Points de vie regeneres par heure entre 75 et 100% des PV.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_potion_soin_price", "label": "Prix Potion de soin", "type": "number", "required": false, "default": "80", "description": "Prix de la Potion de soin au shop.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_antidote_price", "label": "Prix Antidote", "type": "number", "required": false, "default": "150", "description": "Prix de l Antidote au shop.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_potion_majeure_price", "label": "Prix Potion majeure", "type": "number", "required": false, "default": "200", "description": "Prix de la Potion majeure au shop.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_bouclier_price", "label": "Prix Bouclier", "type": "number", "required": false, "default": "250", "description": "Prix du Bouclier au shop.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_poison_price", "label": "Prix Poison", "type": "number", "required": false, "default": "300", "description": "Prix du Poison au shop.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_masque_braquage_price", "label": "Prix Masque de braquage", "type": "number", "required": false, "default": "100", "description": "Prix du Masque (braquage).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_pied_de_biche_price", "label": "Prix Pied-de-biche", "type": "number", "required": false, "default": "150", "description": "Prix du Pied-de-biche (braquage).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_crochet_vault_price", "label": "Prix Crochet de coffre", "type": "number", "required": false, "default": "220", "description": "Prix du Crochet de coffre (braquage).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_plan_coffre_price", "label": "Prix Plan du coffre", "type": "number", "required": false, "default": "320", "description": "Prix du Plan du coffre (braquage).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_fumigene_diversion_price", "label": "Prix Fumigene (diversion)", "type": "number", "required": false, "default": "450", "description": "Prix du Fumigene (braquage).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_explosif_price", "label": "Prix Explosif", "type": "number", "required": false, "default": "600", "description": "Prix de l Explosif (braquage).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_hacker_kit_price", "label": "Prix Kit hacker", "type": "number", "required": false, "default": "800", "description": "Prix du Kit hacker (braquage).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_drone_espion_price", "label": "Prix Drone espion", "type": "number", "required": false, "default": "1000", "description": "Prix du Drone espion (braquage).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "shop_equipe_de_pros_price", "label": "Prix Equipe de pros", "type": "number", "required": false, "default": "1500", "description": "Prix de l Equipe de pros (braquage).", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "shop_potion_soin_price"}]'::jsonb);
