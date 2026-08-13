-- Le casino est volontairement retire de Coup de Coude.
DROP TABLE IF EXISTS nexus_coude_casino_log;
ALTER TABLE nexus_coude_players DROP COLUMN IF EXISTS casino_wins;
ALTER TABLE nexus_coude_players DROP COLUMN IF EXISTS casino_losses;
