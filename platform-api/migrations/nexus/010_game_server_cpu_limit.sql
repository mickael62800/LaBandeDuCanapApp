-- 010_game_server_cpu_limit.sql
--
-- Plafond CPU par serveur de jeu, en nombre de coeurs (2.0 = deux coeurs).
--
-- Jusqu'ici le plafond etait code en dur a 2 vCPU pour TOUS les conteneurs
-- (docker_runtime.rs). Or les besoins different fortement : Minecraft est
-- limite par son thread principal et ne tire quasiment rien au-dela de
-- 2 coeurs, tandis que Palworld est reellement multithreade et en exploite 4.
--
-- NULL = plafond par defaut de l'adapter, comportement inchange.

ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS cpu_limit double precision;

DO $$
BEGIN
    ALTER TABLE game_servers
        ADD CONSTRAINT chk_game_servers_cpu_limit
        CHECK (cpu_limit IS NULL OR (cpu_limit >= 0.5 AND cpu_limit <= 32));
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;
