-- Ajoute les cles d embed enrichi (title, image_url, footer_text) aux 3
-- types de messages welcome (bienvenue, depart, anniversaire) — pour que
-- l admin puisse personnaliser l apparence des embeds via /component-config.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "welcome_title", "label": "Titre de l embed (bienvenue)", "type": "text", "required": false, "default": "Bienvenue !", "description": "Titre en haut de l embed de bienvenue. Ex: \"Ho ! Un nouveau membre !\""},
  {"key": "welcome_image_url", "label": "Image banniere (bienvenue)", "type": "text", "required": false, "default": "", "description": "URL d une image affichee en bas de l embed. Vide = aucune image."},
  {"key": "welcome_footer_text", "label": "Footer de l embed (bienvenue)", "type": "text", "required": false, "default": "{count} membres", "description": "Variable : {count}."},
  {"key": "leave_title", "label": "Titre de l embed (depart)", "type": "text", "required": false, "default": "Au revoir...", "description": "Titre en haut de l embed de depart."},
  {"key": "leave_image_url", "label": "Image banniere (depart)", "type": "text", "required": false, "default": "", "description": "URL d une image affichee en bas de l embed de depart."},
  {"key": "leave_footer_text", "label": "Footer de l embed (depart)", "type": "text", "required": false, "default": "{count} membres", "description": "Variable : {count}."},
  {"key": "anniversary_title", "label": "Titre de l embed (anniversaire)", "type": "text", "required": false, "default": "Joyeux anniversaire !", "description": "Titre en haut de l embed anniversaire."},
  {"key": "anniversary_image_url", "label": "Image banniere (anniversaire)", "type": "text", "required": false, "default": "", "description": "URL d une image affichee en bas de l embed anniversaire."},
  {"key": "anniversary_footer_text", "label": "Footer de l embed (anniversaire)", "type": "text", "required": false, "default": "{count} membres", "description": "Variable : {count}."}
]'::jsonb
WHERE bot_name = 'welcome-bot'
  AND NOT (config_schema @> '[{"key": "welcome_title"}]'::jsonb);
