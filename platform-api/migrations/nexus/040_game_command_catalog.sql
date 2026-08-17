-- 040_game_command_catalog.sql
--
-- Catalogue de commandes d'administration, par modele de jeu.
--
-- La console RCON existait deja, mais en ligne de commande nue : il fallait
-- connaitre par coeur la syntaxe de chaque jeu. Palworld bannit avec
-- `BanPlayer`, Minecraft avec `ban`, 7 Days to Die avec `ban add`. Retenir
-- trois syntaxes, c'est se tromper au moment ou l'on est presse — c'est-a-dire
-- au pire moment.
--
-- Les commandes vivent donc en base, comme `config_schema` : l'ecran se
-- construit a partir d'elles, et ajouter une commande ou un jeu ne demande pas
-- de toucher au front.
--
-- SECURITE : `template` est le gabarit RCON, avec ses emplacements `{cle}`.
-- Il n'est JAMAIS envoye au navigateur. Le web transmet une cle de commande et
-- des parametres ; le serveur retrouve le gabarit, valide chaque valeur et
-- compose la commande lui-meme. Sans cette regle, un bouton « bannir » serait
-- une console RCON ouverte a quiconque sait forger une requete.
--
-- Ce lot ne couvre que Palworld, volontairement : mieux vaut un jeu dont
-- chaque commande a ete verifiee que sept jeux approximatifs. Les autres
-- s'ajouteront par une migration suivante, sans changement de code.

ALTER TABLE game_templates
    ADD COLUMN IF NOT EXISTS command_schema JSONB NOT NULL DEFAULT '[]'::jsonb;

COMMENT ON COLUMN game_templates.command_schema IS
    'Commandes d''administration proposees a l''ecran. Le gabarit `template` reste serveur : le navigateur n''envoie qu''une cle et des parametres.';

-- ── Palworld ──
--
-- Commandes de `PalServer` (protocole RCON standard). `ShowPlayers` renvoie
-- `name,playeruid,steamid` : c'est de la que vient la liste des joueurs
-- connectes proposee dans les champs « joueur » (cf. `presence.rs`).
--
-- `BanPlayer` et `KickPlayer` attendent le SteamID, pas le pseudo : un nom
-- peut changer, contenir des espaces ou imiter celui d'un autre.

UPDATE game_templates SET command_schema = '[
  {"key": "broadcast", "label": "Annoncer un message", "group": "Communication",
   "template": "Broadcast {message}",
   "description": "Affiche un message a tous les joueurs connectes.",
   "params": [
     {"key": "message", "label": "Message", "type": "text", "required": true, "max_length": 120,
      "description": "Les espaces sont acceptes ; les retours a la ligne, non."}
   ]},

  {"key": "kick_player", "label": "Expulser un joueur", "group": "Joueurs",
   "template": "KickPlayer {steamid}",
   "confirm": true,
   "description": "Deconnecte le joueur. Il peut revenir immediatement.",
   "params": [
     {"key": "steamid", "label": "Joueur", "type": "player", "required": true,
      "description": "Identifiant Steam, choisi dans la liste des connectes."}
   ]},

  {"key": "ban_player", "label": "Bannir un joueur", "group": "Joueurs",
   "template": "BanPlayer {steamid}",
   "confirm": true, "danger": true,
   "description": "Interdit definitivement l''acces au serveur.",
   "warning": "Irreversible depuis cette page : lever un bannissement demande d''editer le fichier de bannis du serveur.",
   "params": [
     {"key": "steamid", "label": "Joueur", "type": "player", "required": true}
   ]},

  {"key": "save", "label": "Sauvegarder le monde", "group": "Maintenance",
   "template": "Save",
   "description": "Ecrit la sauvegarde sur le disque. A faire avant toute operation risquee."},

  {"key": "shutdown", "label": "Arreter le serveur", "group": "Maintenance",
   "template": "Shutdown {secondes} {message}",
   "confirm": true, "danger": true,
   "description": "Previent les joueurs, puis arrete le serveur apres le delai.",
   "warning": "Le conteneur s''arrete : il faudra le redemarrer depuis la fiche du serveur.",
   "params": [
     {"key": "secondes", "label": "Delai avant arret", "type": "number", "required": true,
      "min": 1, "max": 600, "description": "En secondes. Laisse aux joueurs le temps de se mettre a l''abri."},
     {"key": "message", "label": "Message d''avertissement", "type": "text", "required": false,
      "max_length": 120}
   ]},

  {"key": "show_players", "label": "Lister les joueurs connectes", "group": "Joueurs",
   "template": "ShowPlayers",
   "description": "Nom, identifiant de joueur et identifiant Steam de chaque personne connectee."},

  {"key": "info", "label": "Version et nom du serveur", "group": "Maintenance",
   "template": "Info"}
]'::jsonb
WHERE slug = 'palworld';
