-- Abandon du role restreint `sentinel_ops` (migration 024).
--
-- POURQUOI IL EST ABANDONNE PLUTOT QUE COMPLETE
--
-- Il n'a jamais ete utilise : `ops-api` s'est toujours connecte avec le compte
-- du cluster. Et ses droits sont FAUX — 024 n'accorde que les vues
-- `ops_logs_v` / `ops_audit_logs_v`, alors qu'ops-api accede a `logs` en
-- direct (INSERT dans `log_repository`, DELETE dans `ip_ban_repository`,
-- SELECT dans `security_log_repository`). Le basculer dessus aurait casse
-- l'ecriture des logs techniques.
--
-- Un role inutilise dont les droits sont incomplets est pire qu'absent : il
-- donne l'impression d'un cloisonnement qui n'existe pas, et il faudrait le
-- maintenir a chaque nouvelle requete d'ops-api.
--
-- Ce qu'il pretendait proteger, c'etait surtout `web_oauth_sessions`. Cette
-- table a quitte la base (cf. 027) : elle vit dans `auth`, derriere un role
-- distinct. Le cloisonnement qui compte est desormais ailleurs — plus aucun
-- service ne se connecte en superuser, donc plus aucun ne peut lire la base
-- d'une autre plateforme.
--
-- Sentinel, l'exploitation et les workers partagent `sentinel_app` : ils
-- travaillent sur la meme base, dans le meme domaine de confiance, et les
-- separer l'un de l'autre n'apporterait rien que la complexite de maintenir
-- deux listes de GRANT.

REVOKE ALL PRIVILEGES ON alert_rules, manual_ip_bans, server_events FROM sentinel_ops;
REVOKE ALL PRIVILEGES ON ops_logs_v, ops_audit_logs_v FROM sentinel_ops;

-- Le role lui-meme n'est pas supprime ici : `DROP ROLE` exige un privilege
-- que `sentinel_app` n'a pas (et c'est tres bien ainsi). Une fois cette
-- migration passee, il ne detient plus aucun droit ; pour le retirer du
-- cluster, en tant que superuser :
--
--   docker compose exec postgres psql -U sentinel -d discord_sentinel \
--     -c "DROP ROLE IF EXISTS sentinel_ops"
