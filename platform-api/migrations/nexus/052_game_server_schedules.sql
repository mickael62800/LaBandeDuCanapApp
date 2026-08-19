-- 052_game_server_schedules.sql
--
-- Plages d'ouverture recurrentes d'un serveur de jeu.
--
-- Un serveur de soiree n'a pas besoin de tourner la journee : il consomme
-- memoire et processeur pour personne, et sur une machine qui heberge
-- plusieurs jeux, c'est autant de pris aux autres.
--
-- Les plages sont saisies en heure LOCALE, avec leur fuseau : l'heure d'ete
-- decale l'UTC d'une heure deux fois par an, et un decalage fige ferait ouvrir
-- le serveur a 18h ou 20h la moitie de l'annee.
--
-- `last_warned_at` porte l'anti-repetition du preavis. Sans lui, les joueurs
-- recevraient le meme « fermeture dans 10 minutes » a chaque passage du job.
-- Persiste, et non garde en memoire : un redemarrage de l'API relancerait
-- sinon l'annonce.

CREATE TABLE IF NOT EXISTS game_server_schedules (
    server_id     UUID PRIMARY KEY REFERENCES game_servers(id) ON DELETE CASCADE,
    enabled       BOOLEAN NOT NULL DEFAULT false,
    -- Nom IANA (« Europe/Paris »), pas un decalage : voir plus haut.
    timezone      TEXT NOT NULL DEFAULT 'Europe/Paris',
    -- [{"start_minute": 720, "end_minute": 840}, ...] — minutes depuis minuit.
    -- Le format appartient au domaine ; le stocker en jsonb evite une
    -- migration a chaque plage ajoutee.
    ranges        JSONB NOT NULL DEFAULT '[]'::jsonb,
    warn_minutes  INTEGER NOT NULL DEFAULT 10 CHECK (warn_minutes BETWEEN 0 AND 120),
    last_warned_at TIMESTAMPTZ,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by    VARCHAR(20)
);

COMMENT ON TABLE game_server_schedules IS
    'Plages d''ouverture quotidiennes. La date de FIN de session vit sur game_servers.closes_at : passee, plus rien ne redemarre.';
COMMENT ON COLUMN game_server_schedules.last_warned_at IS
    'Dernier preavis envoye. Persiste pour qu''un redemarrage de l''API ne relance pas l''annonce.';
