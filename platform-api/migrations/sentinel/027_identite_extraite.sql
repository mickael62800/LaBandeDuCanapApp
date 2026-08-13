-- L'identite quitte la base de Sentinel.
--
-- POURQUOI
--
-- `web_oauth_sessions` et `successful_logins` etaient ici, ce qui faisait de
-- sentinel-api le proprietaire de fait de l'identite : Nexus, Atrium et
-- l'exploitation n'avaient aucun moyen de savoir QUI appelle, et la passerelle
-- nginx devait demander son avis a sentinel-api avant de les servir. Sentinel
-- etait donc la dependance d'execution de tout le back-office — celle qui, en
-- tombant, ferme tout. Meme geste que l'extraction de `ops`.
--
-- Les deux tables vivent desormais dans la base `auth` (auth-api/migrations/
-- 001_init.sql), leurs lignes y ont ete recopiees par le one-shot
-- `auth-data-import` du compose, et `ops-api` lit le journal de login par HTTP
-- au lieu d'un SELECT.
--
-- ORDRE D'APPLICATION — cette migration suppose que la reprise a EU LIEU et
-- qu'elle a ete verifiee. Elle detruit la copie d'origine : appliquee trop
-- tot, elle perd les sessions actives (tout le monde doit se reconnecter) et
-- l'historique des logins.
--
-- Verification avant de lancer, cote base `auth` :
--   SELECT count(*) FROM web_oauth_sessions;
--   SELECT count(*) FROM successful_logins;
-- Les deux doivent au moins egaler les comptes cote discord_sentinel.

DROP TABLE IF EXISTS web_oauth_sessions;
DROP TABLE IF EXISTS successful_logins;
