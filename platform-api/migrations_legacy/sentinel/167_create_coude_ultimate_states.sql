-- Migration 167 : Etats des ultimates par classe (cf. COUPE_AMELIORATIONS 3.1).
--
-- Stocke pour chaque joueur :
--  - `pending_kind` : ultimate active pour le prochain combat (NULL si rien)
--  - `last_used_at` : derniere utilisation (pour cooldown weekly/biweekly)
--
-- 1 row par (guild, user). Cree on-demand lors d un /ultimate.

CREATE TABLE IF NOT EXISTS coude_ultimate_states (
    guild_id      VARCHAR(20) NOT NULL,
    user_id       VARCHAR(20) NOT NULL,
    pending_kind  VARCHAR(20),
    last_used_at  TIMESTAMPTZ,
    activated_at  TIMESTAMPTZ,
    PRIMARY KEY (guild_id, user_id),
    CHECK (pending_kind IS NULL OR pending_kind IN ('bourrin', 'agile', 'fourbe', 'tank'))
);

-- Lookup : ultimates pendantes d un joueur (pour resolve_combat_now).
-- Le PK couvre deja le cas. Index supplementaire si besoin de queries
-- inter-joueurs (laisse vide pour l instant).
