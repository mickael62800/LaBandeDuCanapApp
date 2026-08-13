-- Jeu « Influence » — Phase 4 : information & medias.
-- Boucle : enquete (payante, resolue par le worker) -> information secrete ->
-- revelation = scandale (perte de reputation de la cible). Voir 07.md.

-- Enquetes : une cible (citoyen), un cout, une echeance, un resultat.
CREATE TABLE IF NOT EXISTS influence_investigations (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id          TEXT NOT NULL,
    initiator_id      UUID NOT NULL REFERENCES influence_citizens(id) ON DELETE CASCADE,
    initiator_user_id TEXT NOT NULL,          -- discord id (pour notifier)
    target_user_id    TEXT NOT NULL,
    target_username   TEXT NOT NULL DEFAULT '',
    subject           TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'en_cours',  -- en_cours|reussie|echouee
    resolves_at       TIMESTAMPTZ NOT NULL,
    info_id           UUID,                    -- information produite si reussie
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_influence_investigations_due
    ON influence_investigations (resolves_at) WHERE status = 'en_cours';

-- Informations : intel detenu par un citoyen (secret jusqu'a revelation).
CREATE TABLE IF NOT EXISTS influence_information (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    owner_id        UUID NOT NULL REFERENCES influence_citizens(id) ON DELETE CASCADE,
    target_user_id  TEXT NOT NULL DEFAULT '',
    target_username TEXT NOT NULL DEFAULT '',
    content         TEXT NOT NULL,
    visibility      TEXT NOT NULL DEFAULT 'secret',   -- secret|public
    veracity        TEXT NOT NULL DEFAULT 'vrai',     -- vrai|faux|rumeur
    revealed        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_influence_information_owner
    ON influence_information (owner_id, created_at DESC);

-- Config Phase 4.
UPDATE bot_definitions SET config_schema = config_schema || '[
    {"key":"influence_investigation_cost","label":"Cout d une enquete (Argent)","type":"number","required":false,"default":"300","description":"Argent preleve pour lancer une enquete."},
    {"key":"influence_investigation_hours","label":"Duree d une enquete (heures)","type":"number","required":false,"default":"6","description":"Delai avant le resultat d une enquete."},
    {"key":"influence_investigation_success_pct","label":"Chance de reussite d une enquete (%)","type":"number","required":false,"default":"60","description":"Probabilite qu une enquete aboutisse."},
    {"key":"influence_scandal_reputation_loss","label":"Reputation perdue lors d un scandale","type":"number","required":false,"default":"200","description":"Reputation retiree a la cible d une revelation."}
]'::jsonb
WHERE bot_name = 'influence-bot'
  AND NOT (config_schema @> '[{"key":"influence_investigation_cost"}]'::jsonb);
