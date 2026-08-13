-- Cf. COUPE_AMELIORATIONS 4.5 — Mode duel amical.
--
-- Compte les victoires/defaites en mode amical separement des stats
-- "officielles". Le solde coins n'est jamais touche par un duel amical,
-- seuls ces deux compteurs et l'XP gagne.

ALTER TABLE coude_players
    ADD COLUMN IF NOT EXISTS friendly_wins INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS friendly_losses INTEGER NOT NULL DEFAULT 0;
