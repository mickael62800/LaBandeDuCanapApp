-- 065_sauvegarde_au_redemarrage.sql
--
-- Archive le monde d'un serveur de jeu lors des redemarrages programmes.
--
-- POURQUOI CE MOMENT-LA. Le redemarrage est le seul instant ou le monde est
-- complet sur le disque sans ecriture en cours : le jeu vient de repondre a la
-- commande de sauvegarde, et le conteneur est arrete. Une copie prise a chaud
-- peut contenir un fichier a moitie ecrit — ce qui ne se decouvre qu'au moment
-- de restaurer, c'est-a-dire au pire moment. Aucune tache periodique ne peut
-- reproduire cette fenetre.
--
-- POURQUOI UN INTERVALLE. Une permanence redemarre toutes les trois heures. Sans
-- delai minimal, cela ferait huit archives quasi identiques par jour, soit une
-- quarantaine de gigaoctets quotidiens pour un seul serveur Palworld. A 24 h, on
-- obtient une archive par jour, prise a froid, ce qui est le bon compromis.
--
-- Les archives sont consignees dans `game_backups`, table posee par la migration
-- 007 et restee vide depuis : rien ne l'alimentait.
--
-- Idempotente : chaque cle n'est ajoutee que si elle est absente.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "backup_on_restart", "type": "boolean", "label": "Sauvegarder le monde au redemarrage",
   "default": "true", "required": false,
   "group": "Sauvegardes",
   "depends_on": {"key": "enabled", "equals": "true"},
   "description": "Archive le monde pendant le redemarrage programme, conteneur arrete. Allonge le redemarrage du temps de la copie (environ 20 s pour 5 Go), mais c est le seul moment ou la copie est certaine d etre coherente."}
]'::jsonb
WHERE bot_name = 'game-portal'
  AND NOT jsonb_path_exists(config_schema, '$[*] ? (@.key == "backup_on_restart")');

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "backup_min_interval_hours", "type": "number", "label": "Heures minimum entre deux archives",
   "default": "24", "min": 1, "max": 720, "unit": "h", "required": false,
   "group": "Sauvegardes",
   "depends_on": {"key": "backup_on_restart", "equals": "true"},
   "description": "Un serveur en permanence redemarre plusieurs fois par jour. Sans ce delai, chaque redemarrage produirait une archive de plusieurs gigaoctets quasi identique a la precedente."}
]'::jsonb
WHERE bot_name = 'game-portal'
  AND NOT jsonb_path_exists(config_schema, '$[*] ? (@.key == "backup_min_interval_hours")');

-- Retrouver rapidement la derniere archive automatique d'un serveur : c'est la
-- requete que le redemarrage execute pour decider s'il doit archiver, donc a
-- chaque passage du job de permanence.
CREATE INDEX IF NOT EXISTS idx_game_backups_auto
    ON game_backups (server_id, created_at DESC)
    WHERE backup_type = 'auto';
