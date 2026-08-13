-- Jeu « Influence » — Phase 2 : conversions de capitaux + registre des mouvements.
-- Le cœur du gameplay (04.md §2/§10) : transformer un capital en un autre.
-- Chaque variation de capital est tracee dans un registre append-only.

CREATE TABLE IF NOT EXISTS influence_capital_movements (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT   NOT NULL,
    citizen_id  UUID   NOT NULL REFERENCES influence_citizens(id) ON DELETE CASCADE,
    capital     TEXT   NOT NULL,      -- influence|money|reputation|information|network
    delta       BIGINT NOT NULL,      -- signe : + credit, - debit
    reason      TEXT   NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_influence_movements_citizen
    ON influence_capital_movements (citizen_id, created_at DESC);

-- Taux de conversion (cout en capital SOURCE pour 1 unite de capital CIBLE).
UPDATE bot_definitions SET config_schema = config_schema || '[
    {"key":"influence_conv_money_reputation","label":"Conversion Argent -> Reputation (cout par point)","type":"number","required":false,"default":"10","description":"Argent depense pour gagner 1 point de Reputation (publicite)."},
    {"key":"influence_conv_reputation_influence","label":"Conversion Reputation -> Influence (cout par point)","type":"number","required":false,"default":"5","description":"Reputation depensee pour gagner 1 point d Influence."},
    {"key":"influence_conv_money_information","label":"Conversion Argent -> Information (cout par point)","type":"number","required":false,"default":"20","description":"Argent depense pour gagner 1 point d Information (recherche)."}
]'::jsonb
WHERE bot_name = 'influence-bot'
  AND NOT (config_schema @> '[{"key":"influence_conv_money_reputation"}]'::jsonb);
