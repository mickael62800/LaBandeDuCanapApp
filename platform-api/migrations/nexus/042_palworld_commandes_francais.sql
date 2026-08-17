-- 042_palworld_commandes_francais.sql
--
-- Reecrit le catalogue Palworld en francais correct.
--
-- Les migrations de ce depot s'ecrivent sans accents, et cette convention
-- avait deborde sur les libelles : « Arreter le serveur », « Delai avant
-- arret », « irreversible ». Or ces chaines-la ne sont pas du code — elles
-- s'affichent telles quelles a l'ecran. Un bouton mal accentue donne
-- l'impression d'un outil bricole, et se lit moins vite.
--
-- La regle vaut pour tout ce qui est VU : libelles, descriptions,
-- avertissements, noms de sections. Les commentaires SQL, eux, restent sans
-- accents comme le reste du depot.
--
-- Le catalogue est reecrit en entier plutot que rustine par rustine : le lire
-- d'un bloc est la seule facon de verifier qu'il est homogene, et il fait
-- onze entrees.

UPDATE game_templates SET command_schema = '[
  {"key": "broadcast", "label": "Annoncer un message", "group": "Communication",
   "template": "Broadcast {message}",
   "description": "Affiche un message à tous les joueurs connectés.",
   "params": [
     {"key": "message", "label": "Message", "type": "text", "required": true, "max_length": 120,
      "description": "Les espaces sont acceptés ; les retours à la ligne, non."}
   ]},

  {"key": "show_players", "label": "Lister les joueurs connectés", "group": "Joueurs",
   "template": "ShowPlayers",
   "description": "Nom, identifiant de joueur et identifiant Steam de chaque personne connectée."},

  {"key": "kick_player", "label": "Expulser un joueur", "group": "Joueurs",
   "template": "KickPlayer {steamid}",
   "confirm": true,
   "description": "Déconnecte le joueur. Il peut revenir immédiatement.",
   "params": [
     {"key": "steamid", "label": "Joueur", "type": "player", "required": true,
      "description": "Identifiant Steam, choisi dans la liste des connectés."}
   ]},

  {"key": "ban_player", "label": "Bannir un joueur", "group": "Joueurs",
   "template": "BanPlayer {steamid}",
   "confirm": true, "danger": true,
   "description": "Interdit définitivement l''accès au serveur.",
   "warning": "Se lève uniquement avec « Lever un bannissement », et il faut alors connaître l''identifiant Steam par cœur.",
   "params": [
     {"key": "steamid", "label": "Joueur", "type": "player", "required": true}
   ]},

  {"key": "unban_player", "label": "Lever un bannissement", "group": "Joueurs",
   "template": "UnBanPlayer {steamid}",
   "confirm": true,
   "description": "Rend l''accès au serveur à un joueur banni.",
   "params": [
     {"key": "steamid", "label": "Identifiant Steam", "type": "text", "required": true,
      "max_length": 32,
      "description": "À saisir à la main : un joueur banni ne figure évidemment pas dans la liste des connectés."}
   ]},

  {"key": "teleport_to_me", "label": "Faire venir un joueur", "group": "Joueurs",
   "template": "TeleportToMe {steamid}",
   "confirm": true,
   "description": "Téléporte le joueur auprès de toi.",
   "warning": "N''a d''effet que si tu es toi-même connecté en jeu.",
   "params": [
     {"key": "steamid", "label": "Joueur", "type": "player", "required": true}
   ]},

  {"key": "teleport_to_player", "label": "Rejoindre un joueur", "group": "Joueurs",
   "template": "TeleportToPlayer {steamid}",
   "confirm": true,
   "description": "Te téléporte auprès du joueur.",
   "warning": "N''a d''effet que si tu es toi-même connecté en jeu.",
   "params": [
     {"key": "steamid", "label": "Joueur", "type": "player", "required": true}
   ]},

  {"key": "save", "label": "Sauvegarder le monde", "group": "Maintenance",
   "template": "Save",
   "description": "Écrit la sauvegarde sur le disque. À faire avant toute opération risquée."},

  {"key": "info", "label": "Version et nom du serveur", "group": "Maintenance",
   "template": "Info"},

  {"key": "shutdown", "label": "Arrêter avec préavis", "group": "Maintenance",
   "template": "Shutdown {secondes} {message}",
   "confirm": true, "danger": true,
   "description": "Prévient les joueurs, puis arrête le serveur après le délai.",
   "warning": "Le conteneur s''arrête : il faudra le redémarrer depuis la fiche du serveur.",
   "params": [
     {"key": "secondes", "label": "Délai avant arrêt", "type": "number", "required": true,
      "min": 1, "max": 600,
      "description": "En secondes. Laisse aux joueurs le temps de se mettre à l''abri."},
     {"key": "message", "label": "Message d''avertissement", "type": "text", "required": false,
      "max_length": 120}
   ]},

  {"key": "do_exit", "label": "Arrêter immédiatement", "group": "Maintenance",
   "template": "DoExit",
   "confirm": true, "danger": true,
   "description": "Coupe le serveur sur-le-champ, sans préavis pour les joueurs.",
   "warning": "Tout ce qui n''a pas été sauvegardé est perdu. Sauvegarde d''abord, ou préfère un arrêt avec préavis."}
]'::jsonb
WHERE slug = 'palworld';
