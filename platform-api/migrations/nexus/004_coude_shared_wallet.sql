-- Coup de Coude utilise le wallet Nexus partage. Les statistiques de combat
-- restent dans nexus_coude_players, mais aucun solde n'y est maintenu.
DROP INDEX IF EXISTS idx_nexus_coude_players_rank;
ALTER TABLE nexus_coude_players DROP COLUMN IF EXISTS coins;
