-- 032_palworld_presence_auto.sql
--
-- Bascule en attribution AUTOMATIQUE les hauts faits Palworld que l'adaptateur
-- de presence sait verifier (job `palworld-presence`, RCON `ShowPlayers`).
--
-- `ShowPlayers` renvoie le SteamID64 de chaque joueur connecte : la presence
-- est donc une observation verifiable, reliable a un membre Discord par
-- `game_player_links`. Deux hauts faits en decoulent :
--
--   first_launch_palworld    premiere presence constatee sur un serveur ;
--   palworld_massive_session presence simultanee d'au moins `criteria.players`
--                            joueurs identifies.
--
-- Tous les autres hauts faits Palworld restent en 'manual' : un boss vaincu,
-- un elevage ou l'etat d'une base ne sont pas observables par RCON, et le
-- document interdit de les deduire d'un signal qui ne les prouve pas.

UPDATE achievements
SET verification = 'auto'
WHERE game = 'palworld'
  AND code = 'palworld_massive_session';

-- Seuil par defaut du haut fait « grande expedition ». Reste ajustable par
-- serveur depuis le dashboard ; le job lit cette valeur, il ne la code pas.
UPDATE achievements
SET criteria = jsonb_set(criteria, '{players}', '8'::jsonb, true)
WHERE game = 'palworld'
  AND code = 'palworld_massive_session'
  AND NOT (criteria ? 'players');
