-- Panneau d'aide v3 : une CATÉGORIE par salon d'audience, sélectionnée via un
-- dropdown (type "category") au lieu d'un ID à taper. Chaque salon (Admin /
-- Modération / Membres) peut être rangé sous sa propre catégorie.
UPDATE bot_definitions
SET config_schema = '[
    {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Publie/maintient les panneaux d aide."},
    {"key": "admin_category_id", "label": "Catégorie — salon Admin", "type": "category", "required": false, "default": "", "description": "Catégorie sous laquelle ranger le salon des commandes Admin. Vide = catégorie \"Aide commandes\" créée par le bot.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "moderation_category_id", "label": "Catégorie — salon Modération", "type": "category", "required": false, "default": "", "description": "Catégorie sous laquelle ranger le salon des commandes Modération. Vide = catégorie par défaut.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "membres_category_id", "label": "Catégorie — salon Membres", "type": "category", "required": false, "default": "", "description": "Catégorie sous laquelle ranger le salon des commandes Membres. Vide = catégorie par défaut.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'help-bot';
