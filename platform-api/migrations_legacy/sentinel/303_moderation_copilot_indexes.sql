-- Copilote de moderation — index de support des agregations (lecture seule).
--
-- Le copilote agrege la jurisprudence depuis `automod_reviews` en filtrant par
-- (guild_id, status <> 'voting', created_at) et par cle de flag JSONB. Ces
-- index accelerent :
--   * la distribution des precedents par categorie de flag (GIN sur flags) ;
--   * le comptage des reviews d'un membre et la categorie dominante.

-- Recherche par cle/valeur dans le JSONB `flags` (dominant_flag_category,
-- aggregate_decided_by_flag utilisent `flags -> key = 'true'::jsonb`).
CREATE INDEX IF NOT EXISTS idx_automod_reviews_flags_gin
    ON automod_reviews USING GIN (flags);

-- Historique/reviews ouvertes par membre (count_open_reviews,
-- dominant_flag_category filtrent sur guild_id + user_id + status + created_at).
CREATE INDEX IF NOT EXISTS idx_automod_reviews_guild_user_status
    ON automod_reviews (guild_id, user_id, status, created_at DESC);
