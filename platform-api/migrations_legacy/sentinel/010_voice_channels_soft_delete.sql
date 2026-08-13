-- Soft-delete : garder l'historique des salons vocaux au lieu de les supprimer
ALTER TABLE voice_channels ADD COLUMN IF NOT EXISTS channel_status VARCHAR(10) NOT NULL DEFAULT 'open';
ALTER TABLE voice_channels ADD COLUMN IF NOT EXISTS closed_at TIMESTAMPTZ;

CREATE INDEX idx_voice_channels_status ON voice_channels (channel_status);
