-- Panneau d'aide v2 : réparti en 3 salons par audience (Admin / Modération /
-- Membres), rangés sous une CATÉGORIE Discord. La config passe donc de
-- `channel_id` (salon unique) à `category_id` (catégorie parente des 3 salons).
UPDATE bot_definitions
SET config_schema = '[
    {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Publie/maintient les panneaux d aide."},
    {"key": "category_id", "label": "Catégorie parente", "type": "text", "required": false, "default": "", "description": "ID de la catégorie Discord sous laquelle ranger les salons d aide (Admin/Modération/Membres). Vide = le bot crée une catégorie dédiée.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'help-bot';
