CREATE TABLE IF NOT EXISTS logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    level VARCHAR(10) NOT NULL DEFAULT 'info',
    bot VARCHAR(100) NOT NULL DEFAULT '',
    server VARCHAR(200) NOT NULL DEFAULT '',
    message TEXT NOT NULL
);

CREATE INDEX idx_logs_timestamp ON logs (timestamp DESC);
CREATE INDEX idx_logs_level ON logs (level);
CREATE INDEX idx_logs_bot ON logs (bot);
