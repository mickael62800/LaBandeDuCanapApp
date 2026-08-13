-- Phase analytics — fix bug "1 jour sur 2".
--
-- Le calcul `daily_activity[today].messages = total_user_stats - daily_activity[hier].messages`
-- mélangeait un total cumulatif all-time avec un delta journalier, produisant des
-- valeurs alternées (énormes / petites) dès le 3e jour.
--
-- Solution : table baseline qui fige le `total_user_stats` au début de chaque
-- "jour analytics" (configurable via `analytics.baseline_anchor_hour`, défaut 0 = minuit UTC).
-- Le calcul devient `daily_activity[D].messages = total_now - baseline[D].total_messages`.
-- Une fois le jour passé, baseline[D] reste figée, donc daily_activity[D] aussi.
CREATE TABLE IF NOT EXISTS analytics_daily_baseline (
    guild_id            TEXT        NOT NULL,
    day                 DATE        NOT NULL,
    total_messages      BIGINT      NOT NULL DEFAULT 0,
    total_voice_seconds BIGINT      NOT NULL DEFAULT 0,
    captured_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, day)
);

CREATE INDEX IF NOT EXISTS idx_analytics_baseline_day ON analytics_daily_baseline(day DESC);
