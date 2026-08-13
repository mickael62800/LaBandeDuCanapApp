-- Ajout du type de ticket, channel Discord et vocal lie
ALTER TABLE tickets ADD COLUMN IF NOT EXISTS ticket_type TEXT NOT NULL DEFAULT 'autre';
ALTER TABLE tickets ADD COLUMN IF NOT EXISTS channel_id TEXT;
ALTER TABLE tickets ADD COLUMN IF NOT EXISTS voice_channel_id TEXT;
ALTER TABLE tickets ADD COLUMN IF NOT EXISTS invited_user_id TEXT;
