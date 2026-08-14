-- 033_player_link_platform.sql
--
-- Plateforme de compte sur une liaison d'identite de jeu.
--
-- Palworld se joue via Steam ET via le Microsoft Store / Xbox, qui n'ont pas
-- le meme format d'identifiant (SteamID64 vs XUID/Gamertag). Sans cette
-- colonne, le domaine ne peut pas savoir quel format valider.
--
-- Defaut 'steam' : les liaisons deja enregistrees ont ete saisies quand Steam
-- etait la seule option, leur valeur est donc exacte.

ALTER TABLE game_player_links
    ADD COLUMN IF NOT EXISTS platform TEXT NOT NULL DEFAULT 'steam';

ALTER TABLE game_player_links
    DROP CONSTRAINT IF EXISTS chk_game_player_links_platform;

ALTER TABLE game_player_links
    ADD CONSTRAINT chk_game_player_links_platform
    CHECK (platform IN ('steam', 'xbox'));
