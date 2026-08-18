-- 045_game_servers_config_dirty.sql
--
-- Marque un serveur dont la configuration a change depuis la creation de son
-- conteneur.
--
-- Les reglages d'un serveur de jeu sont passes en VARIABLES D'ENVIRONNEMENT
-- Docker, et Docker les fige a la creation du conteneur. Redemarrer ne les
-- relit pas : `docker start` repart avec l'environnement d'origine.
--
-- Consequence, invisible et durable : modifier un reglage dans le dashboard,
-- enregistrer, redemarrer... et le serveur continuait de tourner avec ses
-- valeurs d'origine. L'ecran annoncait « les changements prennent effet au
-- prochain redemarrage », ce qui etait faux. Un administrateur pouvait passer
-- l'eclosion des oeufs de 72 h a 1 h et voir le jeu garder 72 h, sans que rien
-- ne l'explique.
--
-- Ce drapeau dit « le conteneur ne reflete plus la configuration ». Le
-- prochain demarrage le RECREE au lieu de le redemarrer — le volume, donc le
-- monde et les sauvegardes, est conserve.

ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS config_dirty BOOLEAN NOT NULL DEFAULT false;

COMMENT ON COLUMN game_servers.config_dirty IS
    'true = la configuration a change depuis la creation du conteneur ; le prochain demarrage le recree (le volume est conserve).';

-- Les serveurs existants ont ete configures avant ce mecanisme : leur
-- conteneur ne reflete peut-etre deja plus la base. On les marque tous, pour
-- que leur prochain demarrage reparte d'une configuration sure.
UPDATE game_servers SET config_dirty = true WHERE container_id IS NOT NULL;
