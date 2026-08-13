ALTER TABLE logs ADD COLUMN IF NOT EXISTS category VARCHAR(20) NOT NULL DEFAULT 'discord';

-- Reclasser les logs existants
UPDATE logs SET category = 'bot' WHERE bot IN ('automod-bot', 'moderation-bot', 'security-bot', 'stats-bot', 'ticket-bot', 'image-bot', 'voice-bot', 'audit-bot', 'roles-bot');
UPDATE logs SET category = 'worker' WHERE bot IN ('moderation-worker', 'analytics-worker', 'monitoring-worker');

CREATE INDEX IF NOT EXISTS idx_logs_category ON logs(category);
