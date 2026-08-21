-- 059_ark_console_joueurs.sql
--
-- ARK sait dire combien de joueurs sont connectes : on ouvre sa console.
--
-- POURQUOI LUI, ET PAS LES AUTRES. Compter les joueurs suppose trois choses
-- verifiees : que l'image expose une console, qu'on connaisse la variable qui
-- porte son mot de passe, et surtout que le FORMAT de sa reponse soit connu.
-- ARK remplit les trois — `hermsi/ark-server` ouvre RCON de lui-meme, le mot
-- de passe est l'`ADMIN_PASSWORD` deja configurable a l'ecran, et
-- `ListPlayers` repond soit « No Players Connected », soit une liste numerotee
-- avec les identifiants Steam.
--
-- Les deux autres candidats ont ete ecartes, apres verification :
--
--   Factorio  son mot de passe RCON ne passe pas par une variable mais par un
--             fichier (`config/rconpw`), et le format de reponse de
--             `/players online` n'est documente nulle part. Deviner ce format
--             reviendrait a compter zero joueur sur un serveur peuple.
--
--   7 Days    sa console parle TELNET, pas le protocole RCON de Valve. Le
--   to Die    client de la plateforme ne peut pas s'y connecter du tout.
--
-- CE QUE CELA CHANGE POUR UN SERVEUR ARK EXISTANT. Au prochain demarrage, un
-- port RCON lui est alloue et sa console devient joignable ; son
-- `last_player_count` cesse d'etre fige a zero. Consequence a connaitre :
-- l'extinction automatique, qui ne pouvait pas se declencher tant que le
-- comptage etait aveugle, redevient possible — c'est son role, mais elle
-- s'appuie desormais sur une mesure reelle.
--
-- Le risque inverse est couvert cote code : une reponse que le parseur ne
-- reconnait pas ne vaut plus « zero joueur », elle vaut « je ne sais pas », et
-- le worker s'abstient alors d'ecrire (cf. `LecturePresence`).

UPDATE game_templates
SET supports_rcon = true,
    updated_at = now()
WHERE slug = 'ark'
  AND supports_rcon = false;
