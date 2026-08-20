-- 057_epingler_les_nouvelles_images.sql
--
-- Fige au digest les sept jeux ajoutes par les migrations 054 et 055, comme
-- la migration 029 l'avait fait pour les sept premiers. Ils etaient entres au
-- catalogue sur un tag nu.
--
-- POURQUOI CE N'EST PAS UN DETAIL. `pull_image_if_missing` ne retelecharge
-- jamais une image deja presente sur l'hote. Un tag `latest` se fige donc de
-- lui-meme sur ce qui a ete telecharge la premiere fois — sauf qu'a la
-- difference d'un digest, personne ne sait sur QUOI. Deux hotes, ou le meme
-- hote apres un nettoyage d'images, n'executent alors pas le meme serveur, et
-- rien ne le signale. Le digest ne fige pas plus : il fige LISIBLEMENT.
--
-- CE QUE CELA NE FIGE PAS. Toutes ces images sauf une installent le serveur
-- de jeu au demarrage, par steamcmd ou l'equivalent : le digest fige le
-- harnais, pas la version du jeu, qui suit son canal habituel. La mesaventure
-- de Terraria (migration 056) venait d'une image qui, elle, EMBARQUE le
-- binaire du serveur — c'est le cas a surveiller, pas celui-ci.
--
-- Les digests ci-dessous ont ete lus dans les registres le 20 aout 2026, et
-- ecrits par un script : aucun n'a ete recopie a la main.
--
-- La clause `NOT LIKE '%@sha256:%'` rend la migration sans effet sur une fiche
-- deja epinglee — y compris si un exploitant l'a fait de son cote.

UPDATE game_templates SET image = 'escaping/core-keeper-dedicated:latest@sha256:87fa7925596264e1460cb6c7c128e9d39ac5445206e04e587993b5d4afa7c90f', updated_at = now()
WHERE slug = 'core-keeper' AND image NOT LIKE '%@sha256:%';

UPDATE game_templates SET image = 'mornedhels/enshrouded-server:latest@sha256:85978a10f88a85ab0a0aa92e9821d30424895d38bf81fe543532451219c42d0d', updated_at = now()
WHERE slug = 'enshrouded' AND image NOT LIKE '%@sha256:%';

UPDATE game_templates SET image = 'trueosiris/vrising:latest@sha256:9356f98ad56139cccf4cfc7a62331d02e4216e8156d990792f5050252405bc7d', updated_at = now()
WHERE slug = 'vrising' AND image NOT LIKE '%@sha256:%';

UPDATE game_templates SET image = 'renegademaster/zomboid-dedicated-server:latest@sha256:5e3479ea2ef66a4f14686fd3abc3286cf31a82c0e37f737b4b5976ff37da9951', updated_at = now()
WHERE slug = 'project-zomboid' AND image NOT LIKE '%@sha256:%';

UPDATE game_templates SET image = 'andreasgl4ser/necesse-server:latest@sha256:66348641d8f392faf6e9055575a9623fc1e484bf3fdd0196d7ab08bd00fc4f1a', updated_at = now()
WHERE slug = 'necesse' AND image NOT LIKE '%@sha256:%';

UPDATE game_templates SET image = 'ghcr.io/darkmatterproductions/vintagestory:latest@sha256:d2a805b57d0cefdd36b3a6aaca347f98bab810e6e95519936532d947d6b84b1f', updated_at = now()
WHERE slug = 'vintage-story' AND image NOT LIKE '%@sha256:%';

UPDATE game_templates SET image = 'wolveix/satisfactory-server:latest@sha256:e103700ae6ae4c50f19dac80eadb2a805c5b885e179ae2a40850e967bf189efd', updated_at = now()
WHERE slug = 'satisfactory' AND image NOT LIKE '%@sha256:%';
