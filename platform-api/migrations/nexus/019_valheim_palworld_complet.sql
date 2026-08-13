-- 019_valheim_palworld_complet.sql
--
-- Valheim n'avait que 6 reglages, alors que son image en accepte des
-- dizaines. Palworld en avait deja 56 : on ne comble que ce qui manquait
-- reellement, plutot que d'allonger une liste deja fournie.


-- ─────────────────────────────────────────────────────────────────────
-- Valheim
-- ─────────────────────────────────────────────────────────────────────
--
-- Image `lloesche/valheim-server`. Le jeu lui-meme se regle par des
-- « modificateurs de monde » passes dans SERVER_ARGS : ils changent la
-- difficulte bien plus que n'importe quel autre parametre.

UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "SERVER_NAME", "type": "text", "label": "Nom du serveur",
   "group": "Serveur", "default": "Sentinel", "required": false,
   "description": "Nom affiche dans la liste des serveurs du jeu."},

  {"key": "WORLD_NAME", "type": "text", "label": "Nom du monde",
   "group": "Monde", "default": "Dedicated", "required": false,
   "warning": "Le changer demarre un monde VIERGE. L''ancien reste sur le disque mais n''est plus charge."},

  {"key": "SERVER_PASS", "type": "text", "label": "Mot de passe",
   "group": "Serveur", "required": false,
   "warning": "Valheim EXIGE au moins 5 caracteres et refuse de demarrer sinon. Il ne doit pas non plus contenir le nom du serveur."},

  {"key": "SERVER_PORT", "type": "number", "label": "Port UDP",
   "group": "Serveur", "default": "2456", "min": 1024, "max": 65530, "required": false,
   "description": "Le jeu utilise aussi les deux ports suivants."},

  {"key": "SERVER_ARGS", "type": "text", "label": "Arguments supplementaires",
   "group": "Monde", "required": false,
   "description": "Modificateurs de monde, par exemple -modifier combat hard -modifier resources most. C''est ici que se regle la difficulte reelle du jeu.",
   "warning": "Une option mal ecrite empeche le serveur de demarrer, sans message clair."},

  {"key": "SERVER_PUBLIC", "type": "boolean", "label": "Visible publiquement",
   "group": "Serveur", "default": "false", "required": false,
   "description": "Faux = accessible uniquement par adresse directe."},

  {"key": "ADMINLIST_IDS", "type": "text", "label": "Administrateurs (Steam ID)",
   "group": "Acces", "required": false,
   "description": "Identifiants Steam separes par des espaces."},

  {"key": "PERMITTEDLIST_IDS", "type": "text", "label": "Liste blanche (Steam ID)",
   "group": "Acces", "required": false,
   "warning": "Renseignee, elle devient EXCLUSIVE : personne d''autre ne peut se connecter, pas meme toi si tu t''oublies."},

  {"key": "BANNEDLIST_IDS", "type": "text", "label": "Bannis (Steam ID)",
   "group": "Acces", "required": false},

  {"key": "UPDATE_CRON", "type": "text", "label": "Verification des mises a jour",
   "group": "Maintenance", "default": "*/15 * * * *", "required": false,
   "description": "Format cron. Vide pour ne jamais mettre a jour automatiquement."},

  {"key": "RESTART_CRON", "type": "text", "label": "Redemarrage programme",
   "group": "Maintenance", "default": "0 5 * * *", "required": false,
   "description": "Format cron. Un redemarrage quotidien libere la memoire accumulee."},

  {"key": "RESTART_IF_IDLE", "type": "boolean", "label": "Redemarrer seulement si vide",
   "group": "Maintenance", "default": "true", "required": false,
   "description": "Evite de couper une partie en cours pour un redemarrage de routine."},

  {"key": "BACKUPS_DIRECTORY", "type": "text", "label": "Dossier des sauvegardes",
   "group": "Sauvegardes", "default": "/config/backups", "required": false},

  {"key": "BACKUPS_IF_IDLE", "type": "boolean", "label": "Sauvegarder meme serveur vide",
   "group": "Sauvegardes", "default": "false", "required": false,
   "description": "Faux = pas de sauvegarde quand personne ne joue, rien n''ayant change."},

  {"key": "BACKUPS_MAX_AGE", "type": "number", "label": "Age maximum des sauvegardes (jours)",
   "group": "Sauvegardes", "default": "3", "min": 1, "max": 365, "required": false,
   "warning": "Au-dela de ce delai les sauvegardes sont SUPPRIMEES. Un monde corrompu decouvert trop tard n''est plus rattrapable."},

  {"key": "VALHEIM_PLUS_REPO", "type": "text", "label": "Depot Valheim Plus",
   "group": "Mods", "required": false,
   "description": "Laisser vide pour la version officielle."},

  {"key": "BEPINEX_RELEASES_URL", "type": "text", "label": "URL de BepInEx",
   "group": "Mods", "required": false,
   "description": "Laisser vide pour la derniere version."},

  {"key": "MODS", "type": "text", "label": "Mods (URLs)",
   "group": "Mods", "required": false,
   "description": "URLs directes d''archives, separees par virgule. Installees dans le dossier des plugins.",
   "warning": "Chaque joueur doit installer les MEMES mods, dans la meme version, sinon il ne pourra pas se connecter."},

  {"key": "STATUS_HTTP", "type": "boolean", "label": "Page d''etat HTTP",
   "group": "Supervision", "default": "false", "required": false,
   "description": "Expose l''etat du serveur et la liste des joueurs."},

  {"key": "SUPERVISOR_HTTP", "type": "boolean", "label": "Interface de supervision",
   "group": "Supervision", "default": "false", "required": false,
   "warning": "Ouvre une interface d''administration du conteneur. A n''activer que sur un reseau de confiance."},

  {"key": "TZ", "type": "text", "label": "Fuseau horaire",
   "group": "Serveur", "default": "Europe/Paris", "required": false,
   "description": "Determine l''heure des redemarrages et des sauvegardes programmes."}
]'::jsonb
WHERE slug = 'valheim';


-- ─────────────────────────────────────────────────────────────────────
-- Palworld : le complement
-- ─────────────────────────────────────────────────────────────────────
--
-- 56 reglages y figuraient deja. Ce qui manquait tenait a l'acces et a
-- l'exploitation, pas au jeu : region, multiplateforme, liste de bannis,
-- journalisation.

UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "REGION", "type": "text", "label": "Region declaree",
   "group": "Serveur", "required": false,
   "description": "Code ISO du pays, par exemple FR. Sert au classement dans la liste des serveurs."},

  {"key": "CROSSPLAY_PLATFORMS", "type": "text", "label": "Plateformes autorisees",
   "group": "Acces", "default": "(Steam,Xbox,PS5,Mac)", "required": false,
   "description": "Liste entre parentheses. Retirer une plateforme empeche ses joueurs de se connecter."},

  {"key": "BAN_LIST_URL", "type": "text", "label": "Liste de bannis (URL)",
   "group": "Acces", "default": "https://api.palworldgame.com/api/banlist.txt", "required": false,
   "description": "Liste communautaire officielle. Vider pour ne bannir que localement."},

  {"key": "SHOW_PLAYER_LIST", "type": "boolean", "label": "Liste des joueurs publique",
   "group": "Serveur", "default": "true", "required": false},

  {"key": "ALLOW_CONNECT_PLATFORM", "type": "enum", "label": "Plateforme de connexion",
   "group": "Acces", "default": "Steam", "required": false, "options": ["Steam", "Xbox"]},

  {"key": "LOG_FORMAT_TYPE", "type": "enum", "label": "Format des journaux",
   "group": "Supervision", "default": "text", "required": false, "options": ["text", "json"]},

  {"key": "QUERY_PORT", "type": "number", "label": "Port de requete",
   "group": "Serveur", "default": "27015", "min": 1024, "max": 65535, "required": false,
   "description": "Utilise par la liste des serveurs Steam."},

  {"key": "AUTO_UPDATE_ENABLED", "type": "boolean", "label": "Mise a jour automatique",
   "group": "Maintenance", "default": "false", "required": false,
   "warning": "Une mise a jour peut rendre les mods incompatibles du jour au lendemain, et se declenche sans prevenir."},

  {"key": "AUTO_UPDATE_CRON_EXPRESSION", "type": "text", "label": "Heure de la mise a jour",
   "group": "Maintenance", "default": "0 4 * * *", "required": false,
   "description": "Format cron."},

  {"key": "AUTO_UPDATE_WARN_MINUTES", "type": "number", "label": "Preavis avant mise a jour (min)",
   "group": "Maintenance", "default": "15", "min": 0, "max": 120, "required": false,
   "description": "Delai laisse aux joueurs pour se mettre a l''abri."},

  {"key": "AUTO_REBOOT_WARN_MINUTES", "type": "number", "label": "Preavis avant redemarrage (min)",
   "group": "Maintenance", "default": "5", "min": 0, "max": 120, "required": false},

  {"key": "AUTO_REBOOT_EVEN_IF_PLAYERS_ONLINE", "type": "boolean", "label": "Redemarrer meme avec des joueurs",
   "group": "Maintenance", "default": "false", "required": false,
   "warning": "Coupe une partie en cours. A ne laisser actif que si la memoire pose vraiment probleme."},

  {"key": "TZ", "type": "text", "label": "Fuseau horaire",
   "group": "Serveur", "default": "Europe/Paris", "required": false,
   "description": "Determine l''heure des taches programmees."}
]'::jsonb
WHERE slug = 'palworld';


-- ─────────────────────────────────────────────────────────────────────
-- Deduplication
-- ─────────────────────────────────────────────────────────────────────
--
-- `||` concatene sans rien verifier : une cle deja presente se retrouverait
-- en double, et le formulaire afficherait deux fois le meme champ pour une
-- seule valeur enregistree. SERVER_PUBLIC etait dans ce cas, ajoute par la
-- 012 puis a nouveau ici.
--
-- On garde la PREMIERE occurrence de chaque cle : celle des migrations
-- precedentes, deja eprouvee, plutot que la mienne.

UPDATE game_templates t
SET config_schema = (
    SELECT jsonb_agg(elem ORDER BY ord)
    FROM (
        SELECT DISTINCT ON (elem ->> 'key') elem, ord
        FROM jsonb_array_elements(t.config_schema) WITH ORDINALITY AS a(elem, ord)
        ORDER BY elem ->> 'key', ord
    ) AS unique_par_cle
)
WHERE slug IN ('valheim', 'palworld');
