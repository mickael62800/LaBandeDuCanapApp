-- Jeu « Influence » — agence de presse (webhook).
-- Les actualites du jeu (scandales, lois, organisations...) sont publiees dans
-- un salon dedie via un webhook, sous une persona configurable (nom + avatar).

UPDATE bot_definitions SET config_schema = config_schema || '[
    {"key":"press_enabled","label":"Agence de presse (actualites via webhook)","type":"boolean","required":false,"default":"false","description":"Publie les actualites du jeu (scandales, lois, organisations) dans un salon dedie sous une persona de presse."},
    {"key":"press_channel_id","label":"Salon des actualites (presse)","type":"channel","required":false,"default":"","description":"Salon ou l agence de presse publie les actualites du jeu."},
    {"key":"press_name","label":"Nom de l agence de presse","type":"text","required":false,"default":"📰 Journal du serveur","description":"Nom affiche par le webhook de presse."},
    {"key":"press_avatar_url","label":"Avatar de l agence de presse (URL)","type":"text","required":false,"default":"","description":"URL de l image utilisee comme avatar du webhook de presse. Vide = avatar par defaut."}
]'::jsonb
WHERE bot_name = 'influence-bot'
  AND NOT (config_schema @> '[{"key":"press_enabled"}]'::jsonb);
