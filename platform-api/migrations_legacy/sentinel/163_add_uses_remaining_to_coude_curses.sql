-- Migration 163 : ajoute un compteur d utilisations aux curses
-- (cf. COUPE_AMELIORATIONS 5.2 sabotage Empoisonner wallet).
--
-- NULL = curse purement basee sur la duree (cas par defaut, toutes les
-- maledictions classiques). Une valeur entiere = curse consume au fil
-- des declenchements (Empoisonner = 3 gains).

ALTER TABLE coude_curses
    ADD COLUMN IF NOT EXISTS uses_remaining INT;
