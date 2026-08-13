-- Re-attribution des roles aux membres a leur RETOUR (domaine `guild_backup`).
--
-- A la restauration d'un GuildSnapshot, les membres ABSENTS ne peuvent pas
-- recevoir leurs roles. On persiste ici, par membre, la liste des NOUVEAUX
-- identifiants de roles (deja remappes old->new par le bot) a re-attribuer
-- lorsque le membre rejoindra. L'entree est consommee (DELETE ... RETURNING)
-- au premier join pour garantir l'idempotence (un seul re-rolage).

CREATE TABLE IF NOT EXISTS pending_role_grants (
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    -- Tableau JSONB des nouveaux role_id a attribuer (["123", "456"]).
    role_ids JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);
