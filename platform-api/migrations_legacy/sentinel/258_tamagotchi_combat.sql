-- Tamagotchi M2c — parametres du combat (PvP asynchrone, ELO).

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "combat_cooldown_secs", "label": "Cooldown Combat", "type": "number", "required": false, "default": "3600", "min": 0, "max": 86400, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "combat_elo_k", "label": "Facteur K (ELO)", "type": "number", "required": false, "default": "32", "min": 1, "max": 200, "description": "Amplitude des variations d ELO par combat (classique : 32).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "combat_xp_win", "label": "XP victoire", "type": "number", "required": false, "default": "50", "min": 0, "max": 10000, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "combat_xp_loss", "label": "XP defaite", "type": "number", "required": false, "default": "15", "min": 0, "max": 10000, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "combat_w_str", "label": "Poids FORCE", "type": "number", "required": false, "default": "3", "min": 0, "max": 50, "description": "Poids de la FORCE dans la puissance de combat.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "combat_w_vit", "label": "Poids VITALITE", "type": "number", "required": false, "default": "2", "min": 0, "max": 50, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "combat_w_agi", "label": "Poids AGILITE", "type": "number", "required": false, "default": "2", "min": 0, "max": 50, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "combat_random_max", "label": "Part d aleatoire", "type": "number", "required": false, "default": "30", "min": 0, "max": 1000, "description": "Bonus aleatoire (0 a N) ajoute a la puissance de chaque combattant.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'tamagotchi-bot'
  AND NOT (config_schema @> '[{"key": "combat_elo_k"}]'::jsonb);
