-- 071_zomboid_utilisateur_de_l_image.sql
--
-- Project Zomboid ne demarrait pas du tout : tout echouait en permission.
--
--   /home/steam/Zomboid/ip.txt: Permission denied
--   sed: couldn't open temporary file /home/steam/sedkUcJKd: Permission denied
--   /home/root/.local/steamcmd/steamcmd.sh: Permission denied
--   timeout: failed to run command '.../start-server.sh': No such file or directory
--
-- CE QUE DIT LA TROISIEME LIGNE. `/home/root/.local/...` : le processus ne
-- tournait pas en tant que `steam`. La plateforme impose `--user 1000:1000` a
-- tout modele dont `run_as_root` est faux ; l'image utilise un autre
-- identifiant, et le sien resolvait donc un HOME qui n'existe pas. SteamCMD
-- n'a jamais pu s'executer, le serveur n'a jamais ete telecharge, et le script
-- a fini par chercher un binaire absent.
--
-- `run_as_root` porte mal son nom : il ne donne pas root, il RENONCE a imposer
-- un utilisateur et laisse celui de l'image — ici `steam`. C'est ce qu'il faut
-- pour toute image qui installe son serveur au demarrage, comme le font deja
-- Enshrouded, V Rising et Satisfactory.
--
-- Un second changement l'accompagne, sans lequel celui-ci ne suffit pas :
-- docker-agent refuse un conteneur sans utilisateur impose si son image n'est
-- pas dans `DEFAULT_ROOT_IMAGES`. Les deux vont ensemble.

UPDATE game_templates
SET run_as_root = true, updated_at = now()
WHERE slug = 'project-zomboid';
