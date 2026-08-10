-- Vues : contrat de lecture stable pour Exploitation
CREATE OR REPLACE VIEW ops_logs_v AS SELECT * FROM logs;
CREATE OR REPLACE VIEW ops_audit_logs_v AS SELECT * FROM audit_logs;

-- Role : écriture sur ce qu'Exploitation possède, lecture (et suppression pour la purge) ailleurs
DO
$do$
BEGIN
   IF NOT EXISTS (
      SELECT FROM pg_catalog.pg_roles
      WHERE  rolname = 'sentinel_ops') THEN
      CREATE ROLE sentinel_ops LOGIN PASSWORD 'ops_secret';
   END IF;
END
$do$;

GRANT SELECT, INSERT, UPDATE, DELETE ON alert_rules, ip_bans, server_events TO sentinel_ops;
GRANT SELECT, DELETE ON ops_logs_v, ops_audit_logs_v TO sentinel_ops;
