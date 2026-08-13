-- Phase 0 — Observabilité : active pg_stat_statements pour permettre l'audit
-- des queries lentes via Grafana / requête directe.
--
-- ⚠️ Pré-requis : `shared_preload_libraries=pg_stat_statements` doit être
-- activé dans `postgresql.conf` (déjà fait dans `docker-compose.yml`). Sans
-- ça, le `CREATE EXTENSION` échoue avec "could not access file ...".
--
-- Vue principale : `pg_stat_statements` (queries normalisées + stats d'exécution).
-- Reset des stats : `SELECT pg_stat_statements_reset();` (utile au début de
-- chaque phase de la roadmap pour mesurer le delta).

CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
