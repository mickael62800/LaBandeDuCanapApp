-- 036_game_sync_inventory.sql
--
-- Dernier inventaire Discord connu, par guilde.
--
-- Les jeux mentionnables vivent dans deux mondes : la base sait ce que le
-- dashboard a enregistre, Discord sait quels roles et quels messages existent
-- vraiment. Personne ne pouvait comparer les deux : le bot ne lit pas la base,
-- l'API ne parle pas a Discord. Un role supprime a la main ne remontait donc
-- nulle part, et les attributions echouaient sans que rien ne l'explique.
--
-- Le bot depose ici sa photographie de la guilde ; le rapport de divergence se
-- calcule a la demande en la confrontant a l'etat enregistre.
--
-- Une seule ligne par guilde : un inventaire est jetable, seul le dernier
-- compte. Son absence signifie « on ne sait pas », jamais « tout va bien » —
-- c'est ce que le domaine applique en refusant d'affirmer le moindre ecart
-- sans photographie.

CREATE TABLE IF NOT EXISTS game_sync_inventory (
    guild_id   TEXT        PRIMARY KEY,
    -- Roles, messages de panneau vivants et salons illisibles, tels que le bot
    -- les a vus. Le detail appartient au domaine, pas au schema : le stocker
    -- en jsonb evite une migration a chaque indice ajoute a la comparaison.
    inventory  JSONB       NOT NULL,
    taken_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE game_sync_inventory IS
    'Photographie Discord d''une guilde (roles, panneaux) deposee par nexus-bot pour la reconciliation des jeux mentionnables.';
COMMENT ON COLUMN game_sync_inventory.taken_at IS
    'Date de la photographie. Un rapport calcule sur un inventaire perime doit le dire a l''ecran.';
