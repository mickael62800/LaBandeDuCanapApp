-- Migration 165 : Dette d honneur (cf. COUPE_AMELIORATIONS 5.3).
--
-- Compteur par paire (requester, refuser) : chaque fois que `refuser`
-- decline un combat lance par `requester`, le compteur s incremente.
-- Quand il atteint 3, `requester` peut invoquer /honneur @refuser pour
-- forcer un combat que la cible ne peut pas refuser.

CREATE TABLE IF NOT EXISTS coude_refusal_counts (
    guild_id        VARCHAR(20) NOT NULL,
    requester_id    VARCHAR(20) NOT NULL,
    refuser_id      VARCHAR(20) NOT NULL,
    count           INT NOT NULL DEFAULT 0,
    last_refused_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, requester_id, refuser_id)
);

CREATE INDEX IF NOT EXISTS idx_coude_refusal_counts_pair_count
    ON coude_refusal_counts (guild_id, requester_id, count DESC);
