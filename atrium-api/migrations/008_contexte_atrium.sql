-- Contexte de comportement par serveur pour Atrium.
--
-- POURQUOI
--
-- L'accueil et l'apaisement (« conflit ») avaient un ton figé dans le code : le
-- prompt d'accueil nommait même un serveur en dur, et les rappels d'apaisement
-- étaient des chaînes statiques dans le bot. On expose désormais deux réglages
-- de TON par serveur, texte libre, injectés dans le prompt système du modèle :
--   - `welcome_context`  : personnalité/ton de l'accueil et des réponses ;
--   - `conflict_context` : personnalité/ton des rappels d'apaisement.
--
-- Ce sont des consignes de style, PAS des faits : la base de connaissances
-- (RAG) reste la seule source des règles/salons/rôles. Un contexte vide = le
-- comportement par défaut, donc cette migration ne change rien à l'existant.

UPDATE bot_definitions
SET config_schema = config_schema || '[
      {"key": "welcome_context", "type": "textarea", "label": "Contexte d accueil (ton et personnalite)", "default": "", "required": false,
       "depends_on": {"key": "enabled", "equals": "true"},
       "description": "Consigne libre injectee dans le prompt d accueil pour ajuster le ton (ex. tres chaleureux, tutoiement, humour leger). N ajoute pas de faits : la base de connaissances reste la source des regles."},
      {"key": "conflict_context", "type": "textarea", "label": "Contexte d apaisement (conflits)", "default": "", "required": false,
       "depends_on": {"key": "enabled", "equals": "true"},
       "description": "Consigne libre pour le ton des rappels d apaisement quand la moderation signale une tension (ex. ferme mais bienveillant). Vide = rappels par defaut."}
    ]'::jsonb
WHERE bot_name = 'atrium-bot'
  AND NOT (config_schema @> '[{"key": "welcome_context"}]'::jsonb);
