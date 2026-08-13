-- Migration 137 : remplace les abonnements DB par des roles Discord natifs.
-- Chaque jeu est desormais associe a 1 role Discord (cree par le bot via /game-admin create).
-- Les mentions de jeu passent par le ping natif Discord (@RoleDuJeu) au lieu
-- d'une detection regex `#NomDuJeu` + broadcast des abonnes.

-- 1) Ajoute role_id sur games (nullable : les jeux legacy seront rétrofités
--    manuellement en les recreant, ou via une commande admin dediee plus tard).
ALTER TABLE games ADD COLUMN IF NOT EXISTS role_id TEXT;

CREATE INDEX IF NOT EXISTS idx_games_role_id ON games (guild_id, role_id)
    WHERE role_id IS NOT NULL;

-- 2) Supprime la table des abonnements : le modele n'existe plus.
DROP TABLE IF EXISTS game_subscriptions;
