-- 068_abandon_d_annonce_signale.sql
--
-- Quand une session epuise ses tentatives d'annonce, elle cesse d'etre reprise.
-- Jusqu'ici cela ne se voyait que dans les journaux du service : une soiree
-- pouvait rester sans panneau d'inscription sans que personne ne l'apprenne
-- autrement que par les joueurs.
--
-- L'abandon est desormais annonce dans le salon de logs Nexus de la guilde.
--
-- POURQUOI UNE COLONNE. La reprise passe toutes les cinq minutes. Sans trace,
-- elle republierait la meme alerte a chaque passage, indefiniment : le salon de
-- logs deviendrait illisible et l'alerte perdrait tout sens. La colonne dit
-- « celle-ci a deja ete signalee », rien de plus.
--
-- Elle est distincte de `announcement_posted_at` a dessein : une session
-- abandonnee n'a PAS recu son annonce, et le marquer reviendrait a mentir sur
-- ce qui a ete publie.

ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS announcement_abandon_notified_at timestamptz;

COMMENT ON COLUMN game_servers.announcement_abandon_notified_at IS
    'Instant ou l''abandon de l''annonce a ete signale dans le salon de logs. NULL = pas encore signale.';
