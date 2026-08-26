-- 069_reglement_de_session.sql
--
-- Regles de la soiree : ce qu'on attend des joueurs, ce qui est interdit, le
-- mode de jeu retenu. Saisi a la creation du serveur, modifiable ensuite.
--
-- DEUX USAGES, ET UN SEUL EST GENERATIF. Le texte est transmis a Atrium comme
-- CONTEXTE, pour que son annonce sonne juste et n'invite pas a faire ce que le
-- reglement interdit. Mais ce qui s'affiche sous l'annonce est le texte
-- ORIGINAL, reproduit mot pour mot.
--
-- Pourquoi : un reglement reformule par un modele est un reglement qui change
-- de sens sans que personne ne s'en apercoive. « Pas de PvP hors zone » peut
-- devenir « le PvP est decourage » — et c'est le genre de glissement qui se
-- decouvre au premier litige entre joueurs.
--
-- NULLABLE : la plupart des soirees n'ont pas de reglement ecrit, et en exiger
-- un empecherait de creer un serveur pour une partie improvisee.

ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS rules TEXT;

COMMENT ON COLUMN game_servers.rules IS
    'Reglement de la soiree, affiche mot pour mot sous l''annonce. Transmis a Atrium comme contexte, jamais reformule par lui.';
