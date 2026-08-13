-- Ajouter les parametres Coup de Coude v2 (HP, classes, regen) dans le config_schema
UPDATE bot_definitions SET config_schema = config_schema::jsonb || '[
    {"key": "hp_regen_per_hour", "label": "HP regeneres par heure", "type": "number", "required": false, "default": "10"},
    {"key": "hp_min_combat_pct", "label": "HP minimum pour combattre (%)", "type": "number", "required": false, "default": "20"},
    {"key": "class_change_cost", "label": "Cout changement de classe (coins)", "type": "number", "required": false, "default": "500"},
    {"key": "class_change_cooldown_days", "label": "Cooldown changement de classe (jours)", "type": "number", "required": false, "default": "7"},
    {"key": "reset_stats_cost", "label": "Cout reset stats (coins)", "type": "number", "required": false, "default": "300"},
    {"key": "repos_cooldown_hours", "label": "Cooldown repos (heures)", "type": "number", "required": false, "default": "12"},
    {"key": "don_tax_percent", "label": "Taxe sur les dons de coins (%)", "type": "number", "required": false, "default": "10"},
    {"key": "don_coins_cooldown_secs", "label": "Cooldown dons de coins (secondes)", "type": "number", "required": false, "default": "3600"},
    {"key": "combat_max_rounds", "label": "Rounds max par combat (0 = auto)", "type": "number", "required": false, "default": "0"},
    {"key": "season_duration_days", "label": "Duree d''une saison (jours)", "type": "number", "required": false, "default": "90"}
]'::jsonb
WHERE bot_name = 'coude-bot';
