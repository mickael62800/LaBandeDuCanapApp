-- 060_compteur_activite_de_jeu.sql
--
-- Un troisieme salon compteur : les membres qui jouent a QUELQUE CHOSE, pas
-- seulement sur les serveurs de la maison. League of Legends, un solo, un jeu
-- d'un ami : si Discord l'affiche, on peut le compter.
--
-- CE COMPTEUR NE DEPEND PAS DE NEXUS. Les deux premiers lisent l'etat des
-- serveurs ; celui-ci lit l'activite que Discord publie pour chaque membre. Il
-- reste donc juste meme quand aucun serveur ne tourne — et il est le seul des
-- trois a voir les jeux que la maison n'heberge pas.
--
-- DEUX CONDITIONS, TOUTES DEUX HORS DE CETTE MIGRATION.
--
-- 1. Le bot doit avoir le droit de lire les presences : « Presence Intent »
--    coche dans le portail Discord Developer, PUIS `NEXUS_PRESENCE_INTENT=true`
--    dans l'environnement. Dans cet ordre : demander l'intent sans l'avoir
--    autorise fait refuser la connexion du bot, qui ne demarre plus du tout.
--
-- 2. Chaque membre doit partager son activite (« Afficher mon activite de jeu »
--    dans ses parametres Discord). Ceux qui la masquent ne sont jamais
--    comptes, et c'est leur droit — le compteur mesure donc un minimum, pas
--    une verite.
--
-- Tant que la premiere condition n'est pas remplie, le salon n'est pas touche :
-- afficher « 0 en partie » parce qu'on n'a pas le droit de regarder serait
-- mensonger, et le zero resterait fige sans que personne comprenne pourquoi.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "activity_counter_channel_id", "type": "channel",
   "label": "Salon compteur : membres en partie (tous jeux)",
   "required": false,
   "depends_on": {"key": "enabled", "equals": "true"},
   "description": "Salon vocal dont le nom affiche le nombre de membres qui jouent, y compris a des jeux non heberges ici. Vide : compteur eteint.",
   "warning": "Necessite le droit de lire les presences (Presence Intent cote portail Discord, puis NEXUS_PRESENCE_INTENT=true). Ne compte que les membres qui partagent leur activite de jeu."},

  {"key": "activity_counter_format", "type": "text",
   "label": "Format du compteur de membres en partie",
   "default": "🕹️ En partie : {count}", "max_length": 90, "required": false,
   "depends_on": {"key": "enabled", "equals": "true"},
   "description": "{count} est remplace par le nombre de membres en train de jouer."}
]'::jsonb
WHERE bot_name = 'game-portal'
  AND NOT jsonb_path_exists(config_schema, '$[*] ? (@.key == "activity_counter_channel_id")');
