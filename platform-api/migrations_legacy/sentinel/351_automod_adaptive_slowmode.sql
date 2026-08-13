-- Persistance des salons en slowmode adaptatif actif (BUG3) : le tracker etait
-- 100% en memoire -> apres un redemarrage du bot, un salon slowmode restait
-- bloque (le bot ne savait plus le desactiver). On persiste l'ensemble actif
-- pour le recharger au demarrage. Cle par channel_id (globalement unique cote
-- Discord) ; guild_id conserve pour information. Idempotent.
CREATE TABLE IF NOT EXISTS automod_adaptive_slowmode (
    channel_id   TEXT PRIMARY KEY,
    guild_id     TEXT NOT NULL,
    activated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
