-- Ajoute les cles d embed enrichi pour le message de retour (rejoin),
-- separees du message de bienvenue pour que l admin puisse mettre une
-- banniere et un titre differents ("Bienvenue" vs "Bon retour").

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "rejoin_title", "label": "Titre de l embed (retour)", "type": "text", "required": false, "default": "Bon retour !", "description": "Titre affiche quand un membre deja connu revient sur le serveur."},
  {"key": "rejoin_image_url", "label": "Image banniere (retour)", "type": "text", "required": false, "default": "", "description": "URL d une image distincte de celle de bienvenue. Vide = aucune."},
  {"key": "rejoin_footer_text", "label": "Footer de l embed (retour)", "type": "text", "required": false, "default": "{count} membres", "description": "Variable : {count}."}
]'::jsonb
WHERE bot_name = 'welcome-bot'
  AND NOT (config_schema @> '[{"key": "rejoin_title"}]'::jsonb);
