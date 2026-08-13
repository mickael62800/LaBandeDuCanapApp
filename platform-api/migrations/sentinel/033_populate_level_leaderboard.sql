-- Une vue materialisee creee WITH NO DATA ne peut pas recevoir son premier
-- REFRESH en mode CONCURRENTLY. Le worker utilisait pourtant exclusivement ce
-- mode : sur une installation neuve, la vue restait donc definitivement
-- illisible et GET /api/levels/{guild_id}/leaderboard repondait 500.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM pg_catalog.pg_matviews
        WHERE schemaname = 'public'
          AND matviewname = 'mv_level_leaderboard'
          AND NOT ispopulated
    ) THEN
        REFRESH MATERIALIZED VIEW public.mv_level_leaderboard;
    END IF;
END
$$;
