-- Classement mensuel d'activite (texte / vocal / global) publie sur Discord.
--
-- `user_levels` ne stocke que l'XP CUMULEE. Pour un classement par mois
-- calendaire on capture une "baseline" de l'XP cumulee au debut de chaque
-- mois ; le classement du mois = XP actuelle - baseline du debut de mois.
--
-- `partial` : un baseline pose en cours de mois (jour != 1, cas du tout
-- premier demarrage) ne couvre pas le mois entier -> on ne publiera jamais
-- ce mois-la (on attend le 1er baseline complet). Garantit que seuls des
-- mois COMPLETS sont publies (Option A).
CREATE TABLE IF NOT EXISTS user_levels_monthly_snapshot (
    guild_id    TEXT        NOT NULL,
    user_id     TEXT        NOT NULL,
    period_ym   TEXT        NOT NULL,            -- 'YYYY-MM' : mois dont c'est la baseline de debut
    xp_text     BIGINT      NOT NULL DEFAULT 0,
    xp_voice    BIGINT      NOT NULL DEFAULT 0,
    partial     BOOLEAN     NOT NULL DEFAULT FALSE,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_levels_monthly_snapshot UNIQUE (guild_id, user_id, period_ym)
);

CREATE INDEX IF NOT EXISTS idx_levels_monthly_snapshot_period
    ON user_levels_monthly_snapshot (guild_id, period_ym);

-- Config progression-bot : publication du classement mensuel sur Discord.
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "monthly_ranking_enabled", "label": "Publier le classement mensuel (texte/vocal/global) sur Discord", "type": "boolean", "required": false, "default": "false"},
    {"key": "monthly_ranking_channel_id", "label": "Salon de publication du classement mensuel", "type": "channel", "required": false},
    {"key": "monthly_ranking_top_count", "label": "Nombre de membres affiches dans le classement mensuel", "type": "number", "required": false, "default": "10"}
]'::jsonb
WHERE bot_name = 'progression-bot'
  AND NOT (config_schema @> '[{"key": "monthly_ranking_enabled"}]'::jsonb);
