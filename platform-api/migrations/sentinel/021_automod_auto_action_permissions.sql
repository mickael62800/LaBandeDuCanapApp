-- Autorisations fines des sanctions executees sans validation humaine.
-- Les valeurs par defaut conservent le comportement historique du mode auto :
-- lorsque la review est desactivee, chaque action etait automatiquement permise.
UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key":"auto_actions_selective_enabled","type":"boolean","label":"Actions automatiques sélectives","default":"false","required":false,"depends_on":{"key":"enabled","equals":"true"},"description":"Active le contrôle action par action ci-dessous. Quand ce mode est OFF, le comportement historique est conservé : le mode review décide globalement. Quand il est ON, seules les actions cochées sont exécutées automatiquement ; les autres passent en carte de modération."},
  {"key":"auto_warn_enabled","type":"boolean","label":"Autoriser les avertissements automatiques","default":"true","required":false,"depends_on":{"key":"enabled","equals":"true"},"description":"Si OFF, un avertissement suggere est envoye en carte de moderation au lieu d etre publie automatiquement."},
  {"key":"auto_delete_enabled","type":"boolean","label":"Autoriser les suppressions automatiques","default":"true","required":false,"depends_on":{"key":"enabled","equals":"true"},"description":"Si OFF, une suppression suggeree exige la validation des moderateurs. La protection anti-phishing et la suppression de liens explicitement activees gardent leurs propres reglages."},
  {"key":"auto_mute_enabled","type":"boolean","label":"Autoriser les mutes automatiques","default":"true","required":false,"depends_on":{"key":"enabled","equals":"true"},"description":"Si OFF, un mute suggere est envoye en carte de moderation. Si ON, Sentinel peut appliquer directement le timeout configure."},
  {"key":"auto_kick_enabled","type":"boolean","label":"Autoriser les kicks automatiques","default":"false","required":false,"depends_on":{"key":"enabled","equals":"true"},"description":"Si ON et qu un ban est suggere mais non autorise, Sentinel applique un kick comme sanction de repli. Le membre peut revenir avec une nouvelle invitation."},
  {"key":"auto_ban_enabled","type":"boolean","label":"Autoriser les bans automatiques","default":"false","required":false,"depends_on":{"key":"enabled","equals":"true"},"description":"Si OFF, un ban suggere reste une carte de moderation. Laisser OFF est recommande : le ban reste une decision sensible."}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key":"auto_actions_selective_enabled"}]'::jsonb);
