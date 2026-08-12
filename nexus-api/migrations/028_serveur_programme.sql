-- Ajoute l'etat « scheduled » (ouverture programmee) aux statuts autorises
-- d'un serveur de jeu.
--
-- Un serveur `scheduled` a ses salons Discord et son panneau d'inscription
-- crees, mais son conteneur n'est pas encore lance : le worker le demarre
-- ~5 min avant l'heure de revelation de l'IP (job auto-start).

ALTER TABLE game_servers
    DROP CONSTRAINT IF EXISTS chk_game_servers_status;

ALTER TABLE game_servers
    ADD CONSTRAINT chk_game_servers_status CHECK (
        (status)::text = ANY (
            (ARRAY[
                'created'::character varying,
                'scheduled'::character varying,
                'starting'::character varying,
                'running'::character varying,
                'stopping'::character varying,
                'stopped'::character varying,
                'error'::character varying,
                'deleted'::character varying
            ])::text[]
        )
    );
