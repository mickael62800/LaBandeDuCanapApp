-- Configuration IA per-guild : seuils de confiance pour l'inference
CREATE TABLE IF NOT EXISTS ia_config (
    guild_id        TEXT PRIMARY KEY,
    text_enabled    BOOLEAN NOT NULL DEFAULT true,
    text_threshold  DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    vision_enabled  BOOLEAN NOT NULL DEFAULT true,
    vision_threshold DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
