-- Automod — fenêtre d'inactivité de l'agrégation.
--
-- Une carte agrégée ne fusionne un nouvel incident QUE si elle a reçu une
-- infraction récemment (< vote_aggregate_window_minutes, défaut 60 min). Passé
-- ce délai de silence, un nouvel incident ouvre une NOUVELLE carte au lieu de
-- ré-éditer indéfiniment l'ancienne.
--
-- `last_incident_at` : horodatage du dernier incident agrégé sur la carte
-- (mis à jour à chaque fusion). Défaut = NOW() pour les cartes existantes.

ALTER TABLE automod_reviews
    ADD COLUMN IF NOT EXISTS last_incident_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Index partiel : lookup de la carte 'voting' active d'un (guild, user).
CREATE INDEX IF NOT EXISTS idx_automod_reviews_active_agg
    ON automod_reviews (guild_id, user_id, last_incident_at DESC)
    WHERE status = 'voting';

-- Clé de config exposée en page web.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'vote_aggregate_window_minutes'
        UNION ALL SELECT '{
            "key": "vote_aggregate_window_minutes",
            "label": "Fenêtre d''agrégation (minutes d''inactivité)",
            "type": "number",
            "required": false,
            "default": "60",
            "description": "Une carte agrégée cesse de se mettre à jour après ce délai sans nouvelle infraction. Une infraction ultérieure ouvre une nouvelle carte. Défaut : 60 minutes."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'automod-bot';
