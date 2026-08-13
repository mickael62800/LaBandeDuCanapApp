-- Migration 168 : Systeme de Prestige (cf. COUPE_AMELIORATIONS 3.3).
--
-- Ajoute un compteur prestige_count a coude_players. Quand un joueur
-- atteint le niveau 25, il peut "Prestige" : reset au niveau 1 mais
-- gagne +5% de gains permanents par prestige (cumul). Cap a 5
-- prestiges (=+25% gains perma).

ALTER TABLE coude_players
    ADD COLUMN IF NOT EXISTS prestige_count INT NOT NULL DEFAULT 0;
