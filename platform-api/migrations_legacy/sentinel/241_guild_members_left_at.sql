-- Suivi du depart d'un membre (lifecycle).
--
-- Quand un user quitte le serveur Discord, on met `left_at = NOW()` plutot
-- que de supprimer la ligne. Permet :
--   - badge "parti" sur les listes (membres, infractions, stats)
--   - filtrage automatique des listes de jeu (wallet, slot, wheel, coude, blackjack)
--   - reset de wallet a 0 (empeche d'etre cible de vols/paris)
--
-- Au retour : `left_at = NULL`, `joined_at = NOW()`. Les donnees non-jeu
-- (infractions, stats, audit) sont conservees, l'historique reste lie via
-- l'ID Discord stable.
ALTER TABLE guild_members ADD COLUMN IF NOT EXISTS left_at TIMESTAMPTZ;

-- Index partiel : seuls les membres encore actifs (le filtre courant pour
-- les listes de jeu et la majorite des queries).
CREATE INDEX IF NOT EXISTS idx_guild_members_active
    ON guild_members(guild_id, user_id)
    WHERE left_at IS NULL;
