-- 043_game_servers_closes_at.sql
--
-- Heure de fin annoncee d'une session de jeu.
--
-- Le formulaire de creation demandait deja une date de fermeture, mais elle ne
-- servait qu'a remplir le calendrier de la communaute : elle n'etait nulle
-- part sur le serveur lui-meme. Impossible, donc, de distinguer les trois
-- situations qu'un conteneur arrete peut recouvrir :
--
--   - la session n'a pas encore commence ;
--   - elle est en pause et va reprendre ;
--   - elle est terminee.
--
-- Les trois ne se racontent pas de la meme facon aux joueurs. Sans heure de
-- fin, les cartes Discord annoncaient « ferme » des qu'un conteneur
-- s'arretait, y compris au beau milieu d'une soiree de jeu.
--
-- C'est cette colonne qui permet la regle d'affichage tenue par
-- `domain::entities::game::session_state`.

ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS closes_at TIMESTAMPTZ;

COMMENT ON COLUMN game_servers.closes_at IS
    'Heure de fin annoncee de la session. NULL = aucune fin prevue : un conteneur arrete est alors annonce ferme, faute de pouvoir promettre une reprise.';
