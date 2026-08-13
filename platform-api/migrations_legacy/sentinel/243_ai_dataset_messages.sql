-- Module AI Dataset : collecte autonome de messages texte pour entrainer
-- des modeles IA. Independant de l'audit/automod/surveillance.
-- Toggle ON/OFF par guild via bot_guild_config (bot_name='ai-dataset-bot',
-- config_key='enabled', valeur 'true'/'false'). Defaut OFF.

CREATE TABLE IF NOT EXISTS ai_dataset_messages (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id      TEXT NOT NULL,
    channel_id    TEXT,
    channel_name  TEXT,
    user_id       TEXT NOT NULL,
    content       TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ai_dataset_messages_guild_created
    ON ai_dataset_messages (guild_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ai_dataset_messages_user
    ON ai_dataset_messages (guild_id, user_id);

-- Enregistre le module dans bot_definitions pour qu'il apparaisse sur la
-- page Configuration des composants avec un toggle "Active".
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema) VALUES
('ai-dataset-bot',
 'Collecte Dataset IA',
 'Collecte tous les messages texte des salons (sauf bots) pour entrainer un modele IA. Desactive par defaut. Activez ponctuellement pour preparer un dataset, puis desactivez et exportez via la page Dataset IA.',
 '[]')
ON CONFLICT (bot_name) DO NOTHING;
