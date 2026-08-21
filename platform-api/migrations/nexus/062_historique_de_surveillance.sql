-- 062_historique_de_surveillance.sql
--
-- Garde les mesures de surveillance au lieu de les jeter.
--
-- ETAT PRECEDENT. Les courbes de l'onglet Surveillance etaient accumulees dans
-- le NAVIGATEUR : un point par minute, trente points gardes. Consequences —
-- la fenetre ne depassait jamais une demi-heure, elle repartait de zero a
-- chaque rechargement de page, et regarder « la journee » aurait suppose de
-- laisser l'onglet ouvert vingt-quatre heures. Sur des serveurs qui redemarrent
-- chaque nuit, c'est precisement la journee qu'on veut voir.
--
-- La mesure, elle, existait deja : le controle de sante interroge Docker et la
-- console du jeu toutes les trente secondes, puis ECRASE le releve precedent
-- dans `game_servers` (migration 050). Cette table conserve ce qui etait perdu.
--
-- VOLUME. Un serveur en ligne produit 2 880 lignes par jour. Cinq serveurs sur
-- une retention de sept jours tiennent en une centaine de milliers de lignes,
-- soit quelques megaoctets — sans commune mesure avec les tables d'audit du
-- depot. La purge (`purge-perf-history`) borne la croissance.
--
-- CE QUI N'EST PAS ICI. Aucune agregation pre-calculee : Postgres sait resumer
-- 2 880 lignes en trente points bien plus vite qu'on ne saurait maintenir des
-- tables de resume coherentes. Le jour ou la lecture couterait trop cher, ce
-- sera une vue materialisee, pas un changement d'ecriture.

CREATE TABLE IF NOT EXISTS game_server_perf_history (
    id BIGSERIAL PRIMARY KEY,
    server_id UUID NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
    sampled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ce que le conteneur consomme.
    cpu_percent REAL,
    memory_used_mb INTEGER,
    memory_limit_mb INTEGER,

    -- Ce que le jeu met a repondre. NULL quand la console n'est pas lisible :
    -- une absence de mesure n'est pas une latence nulle.
    rcon_latency_ms INTEGER,

    -- Debit instantane, deja calcule par difference avec le releve precedent.
    -- Les compteurs CUMULES ne sont pas conserves : ils ne font que monter puis
    -- retombent a zero au redemarrage du conteneur, ce qui ne s'interprete pas.
    net_rx_bytes_per_sec BIGINT,
    net_tx_bytes_per_sec BIGINT,

    -- Joueurs vus par la console. NULL = comptage indisponible, a distinguer
    -- d'un serveur reellement vide (cf. `LecturePresence`).
    player_count INTEGER
);

-- L'index porte la seule question qu'on pose a cette table : les mesures d'UN
-- serveur sur une fenetre de temps, les plus recentes d'abord. Il sert aussi
-- la purge, qui balaie par date.
CREATE INDEX IF NOT EXISTS idx_perf_history_serveur_date
    ON game_server_perf_history (server_id, sampled_at DESC);

COMMENT ON TABLE game_server_perf_history IS
    'Serie temporelle de surveillance d''un serveur de jeu, ecrite par le controle de sante toutes les 30 s. Purgee par `purge-perf-history`.';
COMMENT ON COLUMN game_server_perf_history.rcon_latency_ms IS
    'Temps de reponse du jeu. NULL = console illisible ou absente : ce n''est pas une latence nulle.';
COMMENT ON COLUMN game_server_perf_history.player_count IS
    'Joueurs connectes. NULL = comptage indisponible, a ne pas confondre avec un serveur vide.';
