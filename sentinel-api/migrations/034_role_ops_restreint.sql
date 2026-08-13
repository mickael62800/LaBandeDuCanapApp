-- Droits du role `sentinel_ops` : l'exploitation cesse de se connecter avec le
-- proprietaire de la base.
--
-- POURQUOI CETTE FOIS, ALORS QUE 024 A ETE ABANDONNE EN 028
--
-- L'abandon de 024 etait justifie : ses droits etaient FAUX (il n'accordait
-- que les vues `ops_logs_v` / `ops_audit_logs_v` alors qu'ops-api ecrit dans
-- `logs` en direct), et surtout le role n'etait pas joignable — le pgbouncer
-- commun fixe l'utilisateur SERVEUR dans sa `DATABASE_URL`. Toute connexion
-- qui le traverse arrive donc en `sentinel_app`, quelles que soient les
-- identifiants presentes. Changer `OPS_DATABASE_URL` seul n'aurait rien
-- cloisonne du tout : c'est le piege qui a fait croire 024 applique.
--
-- Trois choses ont change :
--   1. Les droits ci-dessous sont derives de l'inventaire complet du SQL
--      d'`ops-api`, `ops-worker` et `ops-adapters` — pas d'une intention.
--   2. Un pool dedie (`ops-pgbouncer`) porte le nouveau role, comme
--      `nexus-pgbouncer` et `atrium-pgbouncer` portent les leurs.
--   3. Le role est reellement utilise : `OPS_DATABASE_URL` pointe dessus.
--
-- CE QUE CE ROLE NE PEUT PAS FAIRE, ET C'EST LE BUT
--
-- Il n'a AUCUN droit sur les tables Discord : membres, infractions, tickets,
-- messages, configuration des bots, sauvegardes de serveur. Compromettre
-- ops-api donne toujours la machine (c'est son metier, cf. O1), mais plus les
-- donnees de la communaute.
--
-- INVENTAIRE — table par table, verbe par verbe. Toute nouvelle requete
-- d'ops-api hors de cette liste echouera en `permission denied`, ce qui est le
-- bon sens de defaillance : visible immediatement, et jamais silencieux.
--
--   logs            SELECT (security_log_repository, ops-worker/alerts)
--                   INSERT (log_repository : logs techniques des services)
--                   DELETE (purge /security/cleanup, via ops_logs_v)
--   audit_logs      SELECT, DELETE (purge, via ops_audit_logs_v)
--   server_events   SELECT, INSERT (journal d'administration), DELETE (purge)
--   manual_ip_bans  SELECT, INSERT, UPDATE (leve de ban), DELETE (purge)
--   alert_rules     SELECT, UPDATE (activation/seuil ; aucune creation)
--
-- Aucune sequence n'est accordee : ces tables ont des cles `uuid` ou `text`,
-- pas de `serial`.

-- Le role est cree par `sentinel-db-init` (CREATE ROLE exige un privilege que
-- `sentinel_app` n'a pas). S'il manque, cette migration doit echouer bruyamment
-- plutot que de laisser une installation a moitie cloisonnee.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'sentinel_ops') THEN
        RAISE EXCEPTION
            'Role sentinel_ops absent : sentinel-db-init doit tourner avant les migrations';
    END IF;
END
$$;

-- Les vues existent depuis 024 ; 028 leur a seulement retire les droits.
-- `CREATE OR REPLACE` les remet en place sur une installation ou elles
-- auraient ete supprimees a la main.
CREATE OR REPLACE VIEW ops_logs_v AS SELECT * FROM logs;
CREATE OR REPLACE VIEW ops_audit_logs_v AS SELECT * FROM audit_logs;

GRANT USAGE ON SCHEMA public TO sentinel_ops;

GRANT SELECT, INSERT, DELETE ON logs TO sentinel_ops;
GRANT SELECT, DELETE ON audit_logs TO sentinel_ops;
GRANT SELECT, INSERT, DELETE ON server_events TO sentinel_ops;
GRANT SELECT, INSERT, UPDATE, DELETE ON manual_ip_bans TO sentinel_ops;
GRANT SELECT, UPDATE ON alert_rules TO sentinel_ops;

-- Les deux vues sont traversees par le code de purge. Les droits sur les
-- tables sous-jacentes sont verifies contre le PROPRIETAIRE de la vue
-- (`sentinel_app`), mais l'acces a la vue elle-meme doit etre accorde.
GRANT SELECT, DELETE ON ops_logs_v TO sentinel_ops;
GRANT SELECT, DELETE ON ops_audit_logs_v TO sentinel_ops;

-- Filet : aucun droit par defaut sur ce qui sera cree plus tard. Sans ca, une
-- future table heriterait des `DEFAULT PRIVILEGES` eventuels et le
-- cloisonnement se deferait a la migration suivante, sans que personne ne le
-- remarque.
ALTER DEFAULT PRIVILEGES IN SCHEMA public REVOKE ALL ON TABLES FROM sentinel_ops;
