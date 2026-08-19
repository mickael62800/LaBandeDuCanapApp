-- 051_game_server_alerts.sql
--
-- Alertes de supervision d'un serveur de jeu, cote SERVEUR.
--
-- Elles existaient deja, mais entierement dans le navigateur : seuils et URL de
-- webhook dans le `localStorage`, verification a chaque rafraichissement de la
-- page. Deux consequences que rien n'annoncait :
--
--   - fermer l'onglet arretait la surveillance. Une alerte qui ne veille que
--     lorsqu'on regarde ne sert a rien : c'est la nuit, page fermee, qu'un
--     serveur sature ;
--   - l'URL du webhook est un SECRET (qui l'a peut ecrire dans le salon), et
--     elle vivait dans le navigateur, envoyee a Discord depuis lui.
--
-- Elle vit desormais ici, et ne repart JAMAIS vers le navigateur : l'ecran
-- apprend seulement qu'un webhook est configure, jamais lequel.
--
-- Les dates de dernier envoi portent l'anti-spam. Cote serveur elles doivent
-- etre persistees : un redemarrage de l'API remettait sinon le compteur a zero
-- et relancait toutes les alertes d'un coup.

CREATE TABLE IF NOT EXISTS game_server_alerts (
    server_id           UUID PRIMARY KEY REFERENCES game_servers(id) ON DELETE CASCADE,
    -- Secret : jamais renvoye au navigateur.
    webhook_url         TEXT NOT NULL,
    cpu_threshold       INTEGER NOT NULL DEFAULT 85  CHECK (cpu_threshold BETWEEN 1 AND 100),
    ram_threshold       INTEGER NOT NULL DEFAULT 90  CHECK (ram_threshold BETWEEN 1 AND 100),
    -- Temps de reponse du jeu : la mesure qui correspond au lag ressenti.
    latency_threshold_ms INTEGER NOT NULL DEFAULT 500 CHECK (latency_threshold_ms BETWEEN 50 AND 60000),
    last_cpu_alert_at     TIMESTAMPTZ,
    last_ram_alert_at     TIMESTAMPTZ,
    last_latency_alert_at TIMESTAMPTZ,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by          VARCHAR(20)
);

COMMENT ON COLUMN game_server_alerts.webhook_url IS
    'Secret. Ne doit jamais apparaitre dans une reponse HTTP ni dans un log.';
COMMENT ON COLUMN game_server_alerts.last_cpu_alert_at IS
    'Anti-spam persiste : sans cela un redemarrage de l''API relancerait toutes les alertes d''un coup.';
