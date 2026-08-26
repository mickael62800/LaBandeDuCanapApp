-- Contexte de ton pour les annonces de soiree de jeu.
--
-- POURQUOI
--
-- Nexus ouvre une session de jeu et publie un panneau d'inscription. Le
-- message qui PRECEDE ce panneau est desormais redige par Atrium, a partir des
-- faits fournis par Nexus : le jeu, la jauge de joueurs, l'horaire, l'ouverture
-- prevue. Nexus n'ecrit pas une phrase, Atrium n'invente pas un chiffre.
--
-- Ce reglage rejoint `welcome_context` et `conflict_context` : c'est une
-- consigne de TON, pas des faits. La difference avec les deux autres est que
-- l'annonce de jeu n'a AUCUN repli statique — quand Atrium ne peut pas ecrire,
-- rien n'est publie et la reprise retente plus tard. Un contexte vide reste
-- valable : le service pose alors un ton par defaut sobre.

UPDATE bot_definitions
SET config_schema = config_schema || '[
      {"key": "game_context", "type": "textarea", "label": "Contexte des annonces de jeu (ton et personnalite)", "default": "", "required": false,
       "depends_on": {"key": "enabled", "equals": "true"},
       "description": "Consigne libre pour le ton des annonces d ouverture de soiree de jeu (ex. sarcastique, blase, epique). N ajoute pas de faits : le jeu, la jauge de joueurs et les horaires viennent de Nexus, et le modele n a pas le droit d en inventer d autres."}
    ]'::jsonb
WHERE bot_name = 'atrium-bot'
  AND NOT (config_schema @> '[{"key": "game_context"}]'::jsonb);
