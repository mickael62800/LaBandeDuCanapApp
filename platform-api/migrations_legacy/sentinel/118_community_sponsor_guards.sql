-- Migration 118 : community-bot — gardes anti-abus pour /parrain
--
-- Apres le passage de /parrain en mode "ouvert a tous les membres"
-- (option B), ajout de 2 nouveaux seuils configurables pour eviter
-- les abus :
--
-- - sponsor_min_parrain_days : le parrain doit etre sur le serveur
--   depuis au moins N jours (anti compte jetable qui parraine direct).
--   Default 7 jours.
--
-- - sponsor_max_filleul_days : le filleul doit etre sur le serveur
--   depuis moins de N jours (un membre deja integre depuis 6 mois n'a
--   pas besoin d'etre parraine → anti-farming de recompenses).
--   Default 30 jours.
--
-- max_sponsorships existe deja (defaut 3).

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "sponsor_min_parrain_days", "label": "Parrainage — jours minimum sur le serveur (parrain)", "type": "number", "required": false, "default": "7"},
    {"key": "sponsor_max_filleul_days", "label": "Parrainage — jours maximum sur le serveur (filleul)", "type": "number", "required": false, "default": "30"}
]'::jsonb
WHERE bot_name = 'community-bot';
