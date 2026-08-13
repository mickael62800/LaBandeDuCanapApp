-- Automod — parametres du systeme de vote (page Composants).
--
-- Ajoute au config_schema de automod-bot les cles pilotant le vote des
-- moderateurs introduit par la migration 251. Tout est parametrable par
-- serveur depuis la page web.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "vote_enabled", "label": "Vote des moderateurs", "type": "boolean", "required": false, "default": "false", "description": "Si ON, une detection automod ouvre un VOTE des moderateurs (choix de la sanction) au lieu d appliquer directement. L admin finalise ensuite.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "vote_deadline_hours", "label": "Delai de vote", "type": "number", "required": false, "default": "72", "min": 1, "max": 720, "unit": "heures", "description": "Duree pendant laquelle les moderateurs peuvent voter. A l echeance, on compte les votes exprimes.", "depends_on": {"key": "vote_enabled", "equals": "true"}},
    {"key": "vote_quorum", "label": "Quorum minimum", "type": "number", "required": false, "default": "3", "min": 1, "max": 50, "unit": "votes", "description": "Nombre minimum de votes exprimes pour que le verdict soit valable. En dessous, l alerte est ignoree.", "depends_on": {"key": "vote_enabled", "equals": "true"}},
    {"key": "vote_mod_role_id", "label": "Role autorise a voter", "type": "role", "required": false, "description": "Role dont les membres peuvent voter. Vide = toute personne avec la permission Discord Moderer les membres.", "depends_on": {"key": "vote_enabled", "equals": "true"}},
    {"key": "vote_admin_role_id", "label": "Role autorise a finaliser", "type": "role", "required": false, "description": "Role dont les membres peuvent appliquer/clore le verdict via le bouton admin. Vide = permission Administrateur.", "depends_on": {"key": "vote_enabled", "equals": "true"}},
    {"key": "vote_tie_action", "label": "En cas d egalite", "type": "enum", "required": false, "default": "ignore", "options": [{"value": "ignore", "label": "Ignorer (aucune sanction)"}, {"value": "clemente", "label": "Sanction la plus clemente"}, {"value": "severe", "label": "Sanction la plus severe"}], "description": "Que faire quand deux sanctions sont a egalite de voix.", "depends_on": {"key": "vote_enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key": "vote_enabled"}]'::jsonb);
