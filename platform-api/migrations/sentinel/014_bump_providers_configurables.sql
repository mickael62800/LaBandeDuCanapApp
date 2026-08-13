-- Bump : rend les plateformes entierement configurables sur le web (page
-- Composants -> Bump Rewards). Chaque plateforme a son bot_id saisissable +
-- son enabled + son cooldown. Ajoute French GG, Discadia, top.gg, Spacebump
-- (bot_id a renseigner par serveur), et expose les bot_id de Disboard/DiscordL.

UPDATE bot_definitions SET
    config_schema = config_schema || '[
        {"key": "disboard_bot_id", "type": "text", "label": "Disboard — ID du bot", "required": false, "default": "302050872383242240", "depends_on": {"key": "disboard_enabled", "equals": "true"}, "description": "ID du bot Disboard (deja connu ; a changer seulement s il evolue)."},
        {"key": "discordl_bot_id", "type": "text", "label": "DiscordL — ID du bot", "required": false, "default": "528557940811104258", "depends_on": {"key": "discordl_enabled", "equals": "true"}, "description": "ID du bot DiscordL (bump et vote)."},

        {"key": "frenchgg_enabled", "type": "boolean", "label": "Provider French GG actif", "required": false, "default": "false"},
        {"key": "frenchgg_bot_id", "type": "text", "label": "French GG — ID du bot", "required": false, "depends_on": {"key": "frenchgg_enabled", "equals": "true"}, "description": "ID du bot French GG : clic droit sur le bot dans Discord puis Copier l identifiant."},
        {"key": "frenchgg_cooldown_minutes", "type": "number", "unit": "min", "min": 1, "max": 1440, "label": "Cooldown French GG (minutes)", "required": false, "default": "120", "depends_on": {"key": "frenchgg_enabled", "equals": "true"}},

        {"key": "discadia_enabled", "type": "boolean", "label": "Provider Discadia actif", "required": false, "default": "false"},
        {"key": "discadia_bot_id", "type": "text", "label": "Discadia — ID du bot", "required": false, "depends_on": {"key": "discadia_enabled", "equals": "true"}, "description": "ID du bot Discadia : clic droit sur le bot puis Copier l identifiant."},
        {"key": "discadia_cooldown_minutes", "type": "number", "unit": "min", "min": 1, "max": 1440, "label": "Cooldown Discadia (minutes)", "required": false, "default": "1440", "depends_on": {"key": "discadia_enabled", "equals": "true"}},

        {"key": "topgg_enabled", "type": "boolean", "label": "Provider top.gg actif", "required": false, "default": "false"},
        {"key": "topgg_bot_id", "type": "text", "label": "top.gg — ID du bot", "required": false, "depends_on": {"key": "topgg_enabled", "equals": "true"}, "description": "ID du bot top.gg : clic droit sur le bot puis Copier l identifiant."},
        {"key": "topgg_cooldown_minutes", "type": "number", "unit": "min", "min": 1, "max": 1440, "label": "Cooldown top.gg (minutes)", "required": false, "default": "720", "depends_on": {"key": "topgg_enabled", "equals": "true"}},

        {"key": "spacebump_enabled", "type": "boolean", "label": "Provider Spacebump actif", "required": false, "default": "false"},
        {"key": "spacebump_bot_id", "type": "text", "label": "Spacebump — ID du bot", "required": false, "depends_on": {"key": "spacebump_enabled", "equals": "true"}, "description": "ID du bot Spacebump : clic droit sur le bot puis Copier l identifiant."},
        {"key": "spacebump_cooldown_minutes", "type": "number", "unit": "min", "min": 1, "max": 1440, "label": "Cooldown Spacebump (minutes)", "required": false, "default": "360", "depends_on": {"key": "spacebump_enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'bump-bot'
  AND NOT (config_schema @> '[{"key": "frenchgg_bot_id"}]'::jsonb);
