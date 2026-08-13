-- Log des parties de casino pour tracker les gains/pertes quotidiens
CREATE TABLE IF NOT EXISTS coude_casino_log (
    id          BIGSERIAL PRIMARY KEY,
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    amount      BIGINT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_casino_log_user_day ON coude_casino_log (guild_id, user_id, created_at DESC);
