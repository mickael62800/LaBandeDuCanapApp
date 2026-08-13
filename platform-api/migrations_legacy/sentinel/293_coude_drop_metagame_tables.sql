-- Nettoyage du meta-jeu Coup de Coude retire (vendetta, primes collectives
-- "bounty", coalition, ultimate). Le code (commandes, services, repos, ports,
-- handlers HTTP, hooks moteur) a deja ete supprime ; on enleve les tables.
--
-- NOTE : `coude_primes` n'est PAS supprimee — c'est le systeme de primes
-- d'inventaire (claim_primes), toujours utilise en combat.
DROP TABLE IF EXISTS coude_bounty_contributions;
DROP TABLE IF EXISTS coude_bounties;
DROP TABLE IF EXISTS coude_coalition_members;
DROP TABLE IF EXISTS coude_coalitions;
DROP TABLE IF EXISTS coude_vendettas;
DROP TABLE IF EXISTS coude_ultimate_states;
