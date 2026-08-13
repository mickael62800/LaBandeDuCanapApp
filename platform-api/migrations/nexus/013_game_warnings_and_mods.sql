-- 013_game_warnings_and_mods.sql
--
-- Deux choses.
--
-- 1. Mods pour les deux modeles qui n'en avaient pas : Palworld et ARK.
--
-- 2. Un champ `warning` sur les reglages a risque.
--
-- Ce second point est le plus utile. Une `description` explique ce que fait
-- un reglage ; un `warning` previent de ce qu'il CASSE. Les melanger noierait
-- l'avertissement dans le texte courant, alors que c'est precisement ce qu'il
-- ne faut pas rater — le front l'affiche donc distinctement.
--
-- Ce qui merite un avertissement, et rien d'autre : ce qui detruit un monde,
-- ce qui empeche le serveur de demarrer, ce qui ouvre une faille, ou ce qui
-- met la machine a genoux. Un avertissement sur chaque champ ne serait plus
-- lu du tout.


-- ─────────────────────────────────────────────────────────────────────
-- Palworld : mods et reglages avances
-- ─────────────────────────────────────────────────────────────────────

UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "ENABLE_MOD_LOADER", "type": "boolean", "label": "Activer le chargeur de mods",
   "group": "Mods", "default": "false", "required": false,
   "description": "Installe UE4SS, necessaire a tout mod Palworld.",
   "warning": "Chaque joueur doit installer les MEMES mods de son cote, sinon il ne pourra pas se connecter."},

  {"key": "MODS_LIST", "type": "text", "label": "Mods (URLs)",
   "group": "Mods", "required": false,
   "description": "URLs directes d''archives de mods, separees par virgule. Installees au demarrage."},

  {"key": "RESTART_ENABLED", "type": "boolean", "label": "Redemarrage automatique quotidien",
   "group": "Maintenance", "default": "false", "required": false,
   "description": "Palworld consomme de plus en plus de memoire avec le temps ; un redemarrage la libere."},

  {"key": "RESTART_CRON_EXPRESSION", "type": "text", "label": "Heure du redemarrage",
   "group": "Maintenance", "default": "0 5 * * *", "required": false,
   "description": "Format cron. Par defaut 5h du matin."},

  {"key": "BACKUP_CRON_EXPRESSION", "type": "text", "label": "Heure des sauvegardes",
   "group": "Sauvegardes", "default": "0 * * * *", "required": false,
   "description": "Format cron. Par defaut toutes les heures."},

  {"key": "DELETE_OLD_BACKUPS", "type": "boolean", "label": "Supprimer les vieilles sauvegardes",
   "group": "Sauvegardes", "default": "false", "required": false,
   "warning": "Une fois supprimee, une sauvegarde ne se recupere pas. Verifie le nombre de jours conserves avant d''activer."},

  {"key": "OLD_BACKUP_DAYS", "type": "number", "label": "Jours de sauvegardes conserves",
   "group": "Sauvegardes", "default": "30", "min": 1, "max": 365, "required": false}
]'::jsonb
WHERE slug = 'palworld';


-- ─────────────────────────────────────────────────────────────────────
-- ARK : mods et reglages
-- ─────────────────────────────────────────────────────────────────────

UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "MOD_IDS", "type": "text", "label": "Mods Steam Workshop",
   "group": "Mods", "required": false,
   "description": "Identifiants Workshop separes par virgule. Telecharges au demarrage.",
   "warning": "ARK charge ses mods au demarrage : chaque mod ajoute plusieurs minutes au lancement, et les gros packs peuvent depasser le quart d''heure."},

  {"key": "MAP", "type": "select", "label": "Carte", "group": "Monde",
   "default": "TheIsland", "required": false,
   "options": ["TheIsland", "TheCenter", "ScorchedEarth_P", "Ragnarok", "Aberration_P", "Extinction", "Valguero_P", "Genesis", "CrystalIsles", "Gen2", "LostIsland", "Fjordur"],
   "warning": "Changer de carte demarre un monde VIERGE. L''ancien reste sur le disque mais n''est plus charge."},

  {"key": "SERVER_PASSWORD", "type": "text", "label": "Mot de passe du serveur",
   "group": "Acces", "required": false},

  {"key": "ADMIN_PASSWORD", "type": "text", "label": "Mot de passe administrateur",
   "group": "Acces", "required": false,
   "warning": "Donne le controle TOTAL du serveur en jeu. A ne jamais partager, et a changer s''il a circule."},

  {"key": "XP_MULTIPLIER", "type": "number", "label": "Multiplicateur d''experience",
   "group": "Regles du jeu", "default": "1", "min": 1, "max": 100, "required": false},

  {"key": "TAMING_SPEED", "type": "number", "label": "Vitesse d''apprivoisement",
   "group": "Regles du jeu", "default": "1", "min": 1, "max": 100, "required": false},

  {"key": "HARVEST_AMOUNT", "type": "number", "label": "Rendement de recolte",
   "group": "Regles du jeu", "default": "1", "min": 1, "max": 100, "required": false}
]'::jsonb
WHERE slug = 'ark';


-- ─────────────────────────────────────────────────────────────────────
-- Avertissements sur les reglages a risque
-- ─────────────────────────────────────────────────────────────────────
--
-- Applique par cle : le meme reglage porte le meme avertissement dans tous
-- les modeles ou il apparait. Chaque entree du schema est reconstruite avec
-- son `warning`, sans toucher au reste de ses proprietes.

DO $$
DECLARE
    avertissement jsonb := '{
      "SEED":              "Changer la graine regenere un monde VIERGE. Le monde actuel est perdu s''il n''a pas ete sauvegarde.",
      "LEVEL_TYPE":        "Changer le type regenere le monde. Ne le modifie que sur un serveur neuf.",
      "TYPE":              "Change le moteur du serveur. VANILLA n''accepte AUCUN plugin ni mod ; PAPER accepte les plugins ; FORGE et FABRIC des mods, qui doivent aussi etre installes par chaque joueur. Passer de l''un a l''autre peut rendre le monde illisible.",
      "VERSION":           "Revenir a une version anterieure peut corrompre un monde deja ouvert dans une version plus recente.",
      "REMOVE_OLD_MODS":   "Vide le dossier des mods a chaque demarrage. Indispensable quand on change de liste, destructeur si des mods ont ete deposes a la main.",
      "ONLINE_MODE":       "Desactive, n''importe qui peut se connecter sous n''importe quel pseudo, y compris le tien. A ne couper que sur un serveur ferme.",
      "WHITE_LIST":        "Active sans liste renseignee, PERSONNE ne peut se connecter, pas meme toi.",
      "HARDCORE":          "A la mort, le joueur passe en spectateur definitivement. Irreversible pour lui.",
      "DIFFICULTY":        "En mode paisible, les monstres disparaissent et certaines ressources deviennent introuvables.",
      "MAX_TICK_TIME":     "Le chien de garde arrete le serveur quand un tick depasse ce delai. Avec de gros modpacks, mets -1 : sinon le serveur se coupe pendant le chargement.",
      "SIMULATION_DISTANCE": "Le reglage qui pese le PLUS sur le processeur. Au-dela de 12 sur une machine modeste, tout le serveur ralentit.",
      "VIEW_DISTANCE":     "Chaque cran augmente nettement la memoire utilisee. 10 suffit presque toujours.",
      "MAX_PLAYERS":       "Compte environ 1 Go de memoire par tranche de 10 joueurs, en plus du serveur lui-meme.",
      "MEMORY":            "Au-dela de 12 Go, Minecraft ralentit au lieu d''accelerer : le ramasse-miettes met plus de temps a parcourir la memoire.",
      "ENABLE_COMMAND_BLOCK": "Un bloc de commande mal ecrit peut figer le serveur ou detruire le monde.",
      "IS_PVP":            "Les joueurs peuvent s''entretuer et detruire les constructions des autres.",
      "ENABLE_PLAYER_TO_PLAYER_DAMAGE": "Les joueurs peuvent s''entretuer, meme au sein d''une meme guilde.",
      "ENABLE_FRIENDLY_FIRE": "Les degats passent entre membres d''une meme guilde. Source classique d''accidents.",
      "DEATH_PENALTY":      "Le reglage le plus severe fait perdre TOUT l''equipement a la mort, sans possibilite de le recuperer.",
      "AUTO_RESET_GUILD_NO_ONLINE_PLAYERS": "Supprime une guilde entiere, bases comprises, si aucun de ses membres ne se connecte pendant le delai defini.",
      "UPDATE_ON_BOOT":     "Met le jeu a jour a chaque demarrage. Une mise a jour peut rendre les mods incompatibles du jour au lendemain.",
      "MULTITHREADING":     "Ameliore les performances mais reste instable sur certaines machines. A desactiver au premier crash inexplique.",
      "BEPINEX":            "Chaque joueur doit installer les MEMES mods, sinon il ne pourra pas se connecter.",
      "VALHEIM_PLUS":       "Doit etre installe a l''identique par tous les joueurs, sous la meme version.",
      "TSHOCK":             "Remplace le serveur standard. Les sauvegardes restent compatibles, mais certains reglages changent de nom.",
      "SECURE":             "Desactive, le serveur devient vulnerable aux clients modifies.",
      "WORLD_SIZE":         "Ne s''applique qu''a la CREATION du monde. Sans effet sur un monde existant.",
      "AUTOCREATE":         "Cree un monde neuf s''il n''en trouve pas. Verifie le nom de la partie avant d''activer, sous peine d''en creer un a cote de l''existant.",
      "SERVERCONFIG_BuildCreate": "Le mode creatif donne des ressources infinies a tous les joueurs. Irreversible pour l''equilibre d''une partie."
    }'::jsonb;
    modele record;
    nouveau jsonb;
    entree jsonb;
    cle text;
BEGIN
    FOR modele IN SELECT id, config_schema FROM game_templates LOOP
        nouveau := '[]'::jsonb;

        FOR entree IN SELECT * FROM jsonb_array_elements(modele.config_schema) LOOP
            cle := entree ->> 'key';
            -- Un `warning` deja present n'est pas ecrase : il a ete ecrit
            -- pour ce modele-la et le connait mieux que cette table generale.
            IF avertissement ? cle AND NOT (entree ? 'warning') THEN
                entree := entree || jsonb_build_object('warning', avertissement -> cle);
            END IF;
            nouveau := nouveau || jsonb_build_array(entree);
        END LOOP;

        UPDATE game_templates SET config_schema = nouveau WHERE id = modele.id;
    END LOOP;
END $$;
