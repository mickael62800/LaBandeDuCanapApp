-- Moderation-bot — texte « mode d'emploi » de la carte d'appel (parametrable).
-- Explique au membre ce qu'on attend (preuves, faits, respect), ses droits (dont
-- demander qu'un moderateur en conflit d'interet ne participe pas) et ses
-- devoirs. Vide = texte par defaut integre au bot. Meme carte pour l'appel et le
-- ban en sursis.

UPDATE bot_definitions SET config_schema = config_schema || '[
    {"key":"appeal_guidelines","label":"Texte du mode d emploi de l appel","type":"text","required":false,"default":"","description":"Regles affichees dans le salon d appel (preuves attendues, droits/devoirs, conflit d interet). Markdown supporte. Vide = texte par defaut."}
]'::jsonb
WHERE bot_name = 'moderation-bot'
  AND NOT (config_schema @> '[{"key":"appeal_guidelines"}]'::jsonb);
