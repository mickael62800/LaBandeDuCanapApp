-- 063_permanence_et_redemarrages.sql
--
-- Deuxieme facon de piloter un serveur dans le temps : la PERMANENCE.
--
-- Jusqu'ici un serveur ne connaissait que les plages d'ouverture (« 18h-20h »).
-- C'est le bon reglage pour une soiree, pas pour un serveur ou l'on passe a
-- n'importe quelle heure. Ceux-la tournent en continu — mais un jeu qui tourne
-- des jours d'affilee ne rend pas la memoire qu'il prend, et finit par ramer
-- puis par tomber. Le redemarrage periodique est le remede, a condition d'etre
-- annonce.
--
-- ── Pourquoi une colonne `mode` et pas un second interrupteur ──
--
-- Les deux systemes s'excluent : des plages eteignent le serveur la nuit, une
-- permanence le rallume. Les laisser cohabiter aurait cree un etat ou l'un
-- defait ce que l'autre fait, a la minute pres, sans que rien ne le signale.
-- Une seule colonne rend cet etat INEXPRIMABLE : le domaine branche dessus, et
-- il n'y a pas de garde-fou a maintenir.
--
-- ── Pourquoi seulement des diviseurs de 24 ──
--
-- Les creneaux sont ancres sur la journee locale. Avec 3 h : 0h, 3h, 6h... et
-- c'est vrai tous les jours. Avec 5 h, la serie deriverait d'un jour a l'autre
-- (0h, 5h, 10h, 15h, 20h, puis 1h le lendemain) et l'annonce « redemarrage a
-- 20h » cesserait d'etre vraie des le deuxieme jour.
--
-- ── Pourquoi deux marqueurs d'annonce ──
--
-- Le preavis (15 min par defaut) et l'annonce finale (1 min) portent sur le
-- MEME creneau. Un marqueur unique aurait fait avaler la seconde par la
-- premiere : les joueurs auraient ete prevenus un quart d'heure avant, puis
-- coupes sans un mot. Les deux sont persistes, et non gardes en memoire : un
-- redemarrage de l'API relancerait sinon les annonces deja faites.

ALTER TABLE game_server_schedules
    ADD COLUMN IF NOT EXISTS mode TEXT NOT NULL DEFAULT 'ranges'
        CHECK (mode IN ('ranges', 'restart')),
    ADD COLUMN IF NOT EXISTS restart_interval_hours INTEGER
        CHECK (restart_interval_hours IN (1, 2, 3, 4, 6, 8, 12, 24)),
    ADD COLUMN IF NOT EXISTS restart_anchor_minute INTEGER NOT NULL DEFAULT 0
        CHECK (restart_anchor_minute BETWEEN 0 AND 59),
    ADD COLUMN IF NOT EXISTS last_restart_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_final_warned_at TIMESTAMPTZ;

-- Une permanence sans cadence ne redemarrerait jamais : autant refuser la
-- configuration a l'ecriture plutot que de laisser un reglage qui ne fait rien
-- et que personne ne comprend en le relisant.
ALTER TABLE game_server_schedules
    DROP CONSTRAINT IF EXISTS game_server_schedules_permanence_a_une_cadence;
ALTER TABLE game_server_schedules
    ADD CONSTRAINT game_server_schedules_permanence_a_une_cadence
    CHECK (mode <> 'restart' OR restart_interval_hours IS NOT NULL);

COMMENT ON COLUMN game_server_schedules.mode IS
    'ranges = plages d''ouverture ; restart = permanence 24/24 avec redemarrages periodiques. Les deux s''excluent par construction.';
COMMENT ON COLUMN game_server_schedules.restart_interval_hours IS
    'Heures entre deux redemarrages. Diviseurs de 24 uniquement : sinon les creneaux derivent d''un jour a l''autre.';
COMMENT ON COLUMN game_server_schedules.restart_anchor_minute IS
    'Minute de l''heure a laquelle tombent les creneaux. 0 = a l''heure pile.';
COMMENT ON COLUMN game_server_schedules.last_restart_at IS
    'Dernier redemarrage programme execute. Empeche de rejouer le meme creneau a chaque passage du job.';
COMMENT ON COLUMN game_server_schedules.last_final_warned_at IS
    'Derniere annonce a une minute. Distincte de last_warned_at : les deux portent sur le meme creneau.';
