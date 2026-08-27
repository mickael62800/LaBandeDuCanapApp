-- 070_zomboid_complet.sql
--
-- Project Zomboid n'exposait que six reglages sur les vingt et un que son
-- image accepte, sans RCON, sans mods, et sans moyen de dire a la JVM combien
-- de memoire prendre.
--
-- Les noms de variables ci-dessous viennent du README de
-- `Renegade-Master/zomboid-dedicated-server`, pas d'une supposition : une
-- variable mal nommee est un reglage qui ne fait RIEN, et cela ne se voit pas.
--
-- ─────────────────────────────────────────────────────────────────────
-- CE QUE CETTE IMAGE NE PERMET PAS : LE BAC A SABLE
-- ─────────────────────────────────────────────────────────────────────
--
-- Population de zombies, rarete du butin, duree du jour, degats, PVP,
-- electricite et eau : tout cela vit dans `SandboxVars.lua`, un fichier de la
-- sauvegarde, et AUCUNE variable d'environnement n'y touche. Les exposer ici
-- creerait des champs qui n'auraient aucun effet — exactement le genre de
-- reglage qu'on croit avoir change pendant des semaines.
--
-- Les rendre pilotables demanderait d'ecrire ce fichier dans le volume avant
-- le premier demarrage, ce que la plateforme sait techniquement faire
-- (`upload_file_to_container`) mais qui est un travail a part entiere : le
-- fichier compte plus de quatre-vingts variables, et le modifier sur une
-- partie en cours ne prend effet qu'apres un redemarrage complet.
--
-- En attendant, le bac a sable se regle dans le jeu, par le menu
-- d'administration, avec le compte declare ci-dessous.

UPDATE game_templates SET
    -- RCON EST TOUJOURS OUVERT sur cette image : elle n'a pas de variable
    -- d'activation. L'ouvrir cote plateforme donne la console web, le comptage
    -- des joueurs, l'annonce avant fermeture et surtout le `save` avant
    -- archivage — sans quoi une sauvegarde a froid archive un monde que le jeu
    -- n'a pas fini d'ecrire.
    supports_rcon = true,
    supports_mods = true,

    default_env = '{
      "SERVER_NAME": "ZomboidSentinel",
      "ADMIN_USERNAME": "superuser",
      "MAX_PLAYERS": "16",
      "DEFAULT_PORT": "16261",
      "UDP_PORT": "16262",
      "MAP_NAMES": "Muldraugh, KY",
      "GAME_VERSION": "public",
      "PAUSE_ON_EMPTY": "true",
      "PUBLIC_SERVER": "true",
      "AUTOSAVE_INTERVAL": "15",
      "USE_STEAM": "true",
      "STEAM_VAC": "true",
      "MAX_RAM": "4096m",
      "TZ": "Europe/Paris"
    }'::jsonb,

    command_schema = '[
      {"key": "broadcast", "label": "Annoncer un message", "group": "Communication",
       "template": "servermsg \"{message}\"",
       "description": "Affiche un message a tous les joueurs connectes.",
       "params": [
         {"key": "message", "label": "Message", "type": "text", "required": true, "max_length": 120}
       ]},

      {"key": "save", "label": "Sauvegarder le monde", "group": "Monde",
       "template": "save",
       "description": "Ecrit le monde sur le disque. Lance automatiquement avant chaque archivage."},

      {"key": "kick_player", "label": "Expulser un joueur", "group": "Joueurs",
       "template": "kickuser \"{joueur}\"",
       "confirm": true,
       "description": "Deconnecte le joueur. Il peut revenir immediatement.",
       "params": [
         {"key": "joueur", "label": "Joueur", "type": "player", "required": true}
       ]},

      {"key": "ban_player", "label": "Bannir un joueur", "group": "Joueurs",
       "template": "banuser \"{joueur}\"",
       "confirm": true, "danger": true,
       "description": "Bannissement definitif, a lever a la main dans le jeu.",
       "params": [
         {"key": "joueur", "label": "Joueur", "type": "player", "required": true}
       ]},

      {"key": "add_admin", "label": "Donner les droits administrateur", "group": "Joueurs",
       "template": "setaccesslevel \"{joueur}\" admin",
       "confirm": true, "danger": true,
       "description": "Le joueur obtient TOUS les pouvoirs, y compris le menu du bac a sable.",
       "params": [
         {"key": "joueur", "label": "Joueur", "type": "player", "required": true}
       ]},

      {"key": "checkmodsneedupdate", "label": "Verifier les mises a jour des mods", "group": "Mods",
       "template": "checkModsNeedUpdate",
       "description": "Repond dans les journaux du serveur. Un mod perime empeche les joueurs de se connecter."}
    ]'::jsonb,

    config_schema = '[
      {"key": "SERVER_NAME", "type": "text", "label": "Nom du serveur",
       "group": "Serveur", "default": "ZomboidSentinel", "max_length": 64,
       "warning": "Ce nom designe aussi les fichiers de sauvegarde : en changer demarre une partie VIERGE."},

      {"key": "GAME_VERSION", "type": "enum", "label": "Branche du jeu",
       "group": "Serveur", "default": "public", "options": ["public", "unstable"],
       "description": "public est la version stable.",
       "warning": "Changer de branche impose la meme version a TOUS les joueurs."},

      {"key": "TZ", "type": "text", "label": "Fuseau horaire du serveur",
       "group": "Serveur", "default": "Europe/Paris",
       "description": "Nom IANA. Sert aux horodatages des journaux, pas a l heure en jeu."},

      {"key": "SERVER_PASSWORD", "type": "text", "label": "Mot de passe du serveur",
       "group": "Acces", "default": "", "max_length": 32,
       "description": "Vide = serveur libre. Demande a chaque connexion."},

      {"key": "ADMIN_USERNAME", "type": "text", "label": "Compte administrateur",
       "group": "Acces", "default": "superuser", "max_length": 32,
       "description": "Ce compte ouvre le menu du bac a sable une fois en jeu."},

      {"key": "ADMIN_PASSWORD", "type": "text", "label": "Mot de passe administrateur",
       "group": "Acces", "default": "",
       "description": "Vide = l image en genere un et l ecrit dans ses journaux au premier demarrage.",
       "warning": "Ce compte a tous les pouvoirs en jeu."},

      {"key": "PUBLIC_SERVER", "type": "boolean", "label": "Serveur public",
       "group": "Acces", "default": "true",
       "description": "Desactive, seuls les joueurs deja autorises peuvent rejoindre."},

      {"key": "STEAM_VAC", "type": "boolean", "label": "Anticheat VAC",
       "group": "Acces", "default": "true",
       "description": "A laisser actif sauf si des joueurs sont bannis a tort."},

      {"key": "USE_STEAM", "type": "boolean", "label": "Serveur Steam",
       "group": "Acces", "default": "true",
       "warning": "Desactiver empeche les mods Workshop de se telecharger."},

      {"key": "MAX_PLAYERS", "type": "number", "label": "Joueurs maximum",
       "group": "Joueurs", "default": 16, "min": 1, "max": 64,
       "warning": "Chaque joueur ouvre une portion de carte simulee : la memoire monte vite au-dela d une dizaine."},

      {"key": "PAUSE_ON_EMPTY", "type": "boolean", "label": "Suspendre quand le serveur est vide",
       "group": "Joueurs", "default": "true",
       "description": "Le temps ne passe plus quand personne n est connecte. Economise beaucoup de processeur."},

      {"key": "MAP_NAMES", "type": "text", "label": "Cartes chargees",
       "group": "Monde", "default": "Muldraugh, KY",
       "description": "Liste ordonnee separee par des points-virgules. La carte de base se met en DERNIER.",
       "warning": "Modifier cette liste sur une partie en cours corrompt les zones deja explorees."},

      {"key": "AUTOSAVE_INTERVAL", "type": "number", "label": "Sauvegarde automatique (minutes)",
       "group": "Monde", "default": 15, "min": 1, "max": 120,
       "description": "Un intervalle court protege mieux, au prix d une micro-coupure a chaque ecriture."},

      {"key": "MOD_WORKSHOP_IDS", "type": "text", "label": "Identifiants Workshop",
       "group": "Mods", "default": "",
       "description": "Identifiants numeriques separes par des points-virgules (2160432461;2685168362).",
       "warning": "Doit correspondre EXACTEMENT a la liste ci-dessous, dans le meme ordre."},

      {"key": "MOD_NAMES", "type": "text", "label": "Noms des mods",
       "group": "Mods", "default": "",
       "description": "Noms internes separes par des points-virgules, dans l ordre des identifiants.",
       "warning": "Un mod present dans une liste et absent de l autre empeche le serveur de demarrer."},

      {"key": "MAX_RAM", "type": "text", "label": "Memoire de la machine Java",
       "group": "Technique", "default": "4096m",
       "description": "Format 4096m ou 4g. C est CE reglage que le jeu utilise reellement.",
       "warning": "Independant de la memoire allouee au conteneur : augmenter le curseur SANS le changer ici ne donne rien de plus au jeu. Laisser au moins 1 Go d ecart sous la memoire allouee."},

      {"key": "GC_CONFIG", "type": "enum", "label": "Collecteur memoire Java",
       "group": "Technique", "default": "ZGC", "options": ["ZGC", "G1GC", "SerialGC", "ParallelGC"],
       "description": "ZGC convient par defaut. A ne changer qu en cas de micro-saccades repetees."},

      {"key": "UDP_PORT", "type": "number", "label": "Port UDP secondaire",
       "group": "Technique", "default": 16262, "min": 1024, "max": 65535,
       "warning": "Doit rester libre sur l hote. A ne changer qu en cas de conflit."}
    ]'::jsonb,

    updated_at = now()
WHERE slug = 'project-zomboid';
