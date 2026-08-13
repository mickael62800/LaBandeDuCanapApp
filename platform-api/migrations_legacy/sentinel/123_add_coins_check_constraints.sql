-- ============================================
-- Phase audit Coude : filets de securite DB-level sur les soldes de coins.
--
-- Ajoute CHECK (coins >= 0) sur user_wallets et coude_players.
-- Tout futur bug applicatif qui tenterait d'ecrire un solde negatif sera
-- rejete par la DB avec une erreur explicite au lieu de corrompre les stats.
--
-- Les tables filles (total_earned, total_spent, total_lost, total_stolen,
-- etc.) restent sans contrainte : ce sont des compteurs cumulatifs qui
-- peuvent theoriquement grandir sans limite, et les contraindre casserait
-- des calculs intermediaires.
-- ============================================

-- Nettoyage prealable : si la corruption historique existe, on la clamp a 0
-- avant d'ajouter le CHECK (sinon ALTER TABLE echoue sur les lignes fautives).
UPDATE user_wallets SET coins = 0 WHERE coins < 0;
UPDATE coude_players SET coins = 0 WHERE coins < 0;

-- Contraintes CHECK. IF NOT EXISTS sur les constraints n'est pas supporte
-- en standard Postgres, on utilise un DO block qui verifie pg_constraint.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'user_wallets_coins_non_negative'
    ) THEN
        ALTER TABLE user_wallets
            ADD CONSTRAINT user_wallets_coins_non_negative CHECK (coins >= 0);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'coude_players_coins_non_negative'
    ) THEN
        ALTER TABLE coude_players
            ADD CONSTRAINT coude_players_coins_non_negative CHECK (coins >= 0);
    END IF;
END $$;
