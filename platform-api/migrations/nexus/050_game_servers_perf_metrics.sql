-- 050_game_servers_perf_metrics.sql
--
-- Mesures de reactivite d'un serveur de jeu.
--
-- Les joueurs se plaignent de lags, et rien dans le dashboard ne permet de les
-- constater : CPU et RAM disent ce que le conteneur CONSOMME, pas ce que le
-- serveur MET A REPONDRE. Un serveur peut ramer a 30 % de processeur.
--
-- Trois colonnes, trois questions differentes :
--
--   `rcon_latency_ms` — le temps que met le jeu a repondre a une commande.
--   C'est le signal le plus direct, et le seul qui vienne du jeu lui-meme : un
--   serveur qui met deux secondes a repondre a `ShowPlayers` est un serveur
--   qui rame, quelle que soit sa consommation. La mesure est gratuite : le
--   controle de sante fait deja cette requete toutes les 30 secondes.
--
--   `net_rx_bytes` / `net_tx_bytes` / `net_sampled_at` — la mesure precedente
--   des compteurs reseau du conteneur. Docker ne donne que des totaux cumules ;
--   le debit se calcule par difference avec l'echantillon d'avant, ce qui
--   demande de le garder. Un serveur sature en emission fait laguer tout le
--   monde, et cela se voit avant les plaintes.

ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS rcon_latency_ms INTEGER,
    ADD COLUMN IF NOT EXISTS net_rx_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS net_tx_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS net_sampled_at TIMESTAMPTZ;

COMMENT ON COLUMN game_servers.rcon_latency_ms IS
    'Temps de reponse du jeu a la derniere commande de controle. NULL = jamais mesure, ou serveur sans RCON.';
COMMENT ON COLUMN game_servers.net_sampled_at IS
    'Date de l''echantillon reseau precedent. Sert a calculer un debit a partir des compteurs cumules de Docker.';
