-- Phase 2 A.4 — Partitionnement RANGE des tables event-heavy
--
-- Objectif : convertir 4 tables hot en tables partitionnees mensuellement par
-- created_at (ou timestamp). Gains attendus :
--   - Queries temporelles 10-100x plus rapides (partition pruning)
--   - VACUUM par partition au lieu de full-table scan
--   - Purges en O(1) via DROP PARTITION (futur)
--
-- Tables traitees :
--   1. infractions
--   2. audit_logs
--   3. user_activity_log
--   4. logs (cle = `timestamp`, pas `created_at`)
--
-- Tables NON traitees (et pourquoi) :
--   - moderation_actions / security_events : volumes moindres, gain marginal
--   - daily_activity / hourly_activity : pas de created_at, basees sur day
--   - coude_casino_log : BIGSERIAL incompatible avec partitionnement direct
--
-- Approche transactionnelle :
--   1. RENAME ancienne table vers _old
--   2. CREATE nouvelle table partitionnee (PK = (id, partition_key))
--   3. CREATE partitions mensuelles 2026-04 -> 2027-03 + DEFAULT
--   4. INSERT INTO nouvelle SELECT * FROM _old
--   5. Recreer index (auto-partitionnes)
--   6. DROP _old
--
-- Note importante : la PK passe de `(id)` a `(id, created_at)` car Postgres
-- exige que toute contrainte UNIQUE/PK inclue la cle de partition. Cela
-- n'affecte PAS le code Rust : sqlx voit toujours `id` comme PK fonctionnel.

-- ── Helper : DROP des MV qui referencent les tables a recreer ────────────────
-- (Aucune MV de Phase 2 ne reference ces 4 tables, mais on prevoit le futur)

-- ════════════════════════════════════════════════════════════════════════════
-- 1. INFRACTIONS
-- ════════════════════════════════════════════════════════════════════════════

ALTER TABLE infractions RENAME TO infractions_old;

CREATE TABLE infractions (
    id         UUID NOT NULL,
    guild_id   VARCHAR(20) NOT NULL,
    channel_id VARCHAR(20) NOT NULL,
    user_id    VARCHAR(20) NOT NULL,
    username   TEXT NOT NULL,
    message_id VARCHAR(20) NOT NULL,
    content    TEXT NOT NULL,
    flags      JSONB NOT NULL,
    score      DOUBLE PRECISION NOT NULL,
    action     TEXT NOT NULL,
    reason     TEXT NOT NULL,
    duration   BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Partition default pour l'historique anterieur
CREATE TABLE infractions_default PARTITION OF infractions DEFAULT;

-- Partitions mensuelles 12 mois (2026-04 -> 2027-03)
CREATE TABLE infractions_2026_04 PARTITION OF infractions FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE infractions_2026_05 PARTITION OF infractions FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE infractions_2026_06 PARTITION OF infractions FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE infractions_2026_07 PARTITION OF infractions FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE infractions_2026_08 PARTITION OF infractions FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE infractions_2026_09 PARTITION OF infractions FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE infractions_2026_10 PARTITION OF infractions FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');
CREATE TABLE infractions_2026_11 PARTITION OF infractions FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');
CREATE TABLE infractions_2026_12 PARTITION OF infractions FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');
CREATE TABLE infractions_2027_01 PARTITION OF infractions FOR VALUES FROM ('2027-01-01') TO ('2027-02-01');
CREATE TABLE infractions_2027_02 PARTITION OF infractions FOR VALUES FROM ('2027-02-01') TO ('2027-03-01');
CREATE TABLE infractions_2027_03 PARTITION OF infractions FOR VALUES FROM ('2027-03-01') TO ('2027-04-01');

-- Migration des donnees
INSERT INTO infractions SELECT * FROM infractions_old;

-- DROP des anciens index (toujours attaches a infractions_old qui sera
-- supprimee juste apres). Les noms sont globaux dans le schema et doivent
-- etre liberes avant de les recreer sur la nouvelle table partitionnee.
DROP INDEX IF EXISTS idx_infractions_guild;
DROP INDEX IF EXISTS idx_infractions_user;
DROP INDEX IF EXISTS idx_infractions_created;
DROP INDEX IF EXISTS idx_infractions_action;
DROP INDEX IF EXISTS idx_infractions_guild_created;
DROP INDEX IF EXISTS idx_infractions_guild_action;
DROP INDEX IF EXISTS idx_infractions_guild_action_created;
DROP INDEX IF EXISTS idx_infractions_flags_gin;

-- Recreation des index (auto-propagated to partitions)
CREATE INDEX idx_infractions_user ON infractions (guild_id, user_id);
CREATE INDEX idx_infractions_guild_created ON infractions (guild_id, created_at DESC);
CREATE INDEX idx_infractions_guild_action ON infractions (guild_id, action);
CREATE INDEX idx_infractions_guild_action_created ON infractions (guild_id, action, created_at DESC);
CREATE INDEX idx_infractions_flags_gin ON infractions USING GIN (flags);

DROP TABLE infractions_old;

-- ════════════════════════════════════════════════════════════════════════════
-- 2. AUDIT_LOGS
-- ════════════════════════════════════════════════════════════════════════════

ALTER TABLE audit_logs RENAME TO audit_logs_old;

CREATE TABLE audit_logs (
    id           UUID NOT NULL DEFAULT gen_random_uuid(),
    guild_id     VARCHAR(20) NOT NULL,
    event_type   TEXT NOT NULL,
    actor_id     VARCHAR(20),
    actor_name   TEXT,
    target_id    VARCHAR(20),
    target_name  TEXT,
    channel_id   VARCHAR(20),
    channel_name TEXT,
    details      JSONB DEFAULT '{}',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE audit_logs_default PARTITION OF audit_logs DEFAULT;

CREATE TABLE audit_logs_2026_04 PARTITION OF audit_logs FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE audit_logs_2026_05 PARTITION OF audit_logs FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE audit_logs_2026_06 PARTITION OF audit_logs FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE audit_logs_2026_07 PARTITION OF audit_logs FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE audit_logs_2026_08 PARTITION OF audit_logs FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE audit_logs_2026_09 PARTITION OF audit_logs FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE audit_logs_2026_10 PARTITION OF audit_logs FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');
CREATE TABLE audit_logs_2026_11 PARTITION OF audit_logs FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');
CREATE TABLE audit_logs_2026_12 PARTITION OF audit_logs FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');
CREATE TABLE audit_logs_2027_01 PARTITION OF audit_logs FOR VALUES FROM ('2027-01-01') TO ('2027-02-01');
CREATE TABLE audit_logs_2027_02 PARTITION OF audit_logs FOR VALUES FROM ('2027-02-01') TO ('2027-03-01');
CREATE TABLE audit_logs_2027_03 PARTITION OF audit_logs FOR VALUES FROM ('2027-03-01') TO ('2027-04-01');

INSERT INTO audit_logs SELECT * FROM audit_logs_old;

DROP INDEX IF EXISTS idx_audit_logs_guild;
DROP INDEX IF EXISTS idx_audit_logs_event_type;
DROP INDEX IF EXISTS idx_audit_logs_actor;
DROP INDEX IF EXISTS idx_audit_logs_target;
DROP INDEX IF EXISTS idx_audit_logs_created_at;
DROP INDEX IF EXISTS idx_audit_logs_guild_created;
DROP INDEX IF EXISTS idx_audit_logs_guild_type_date;

CREATE INDEX idx_audit_logs_event_type ON audit_logs (event_type);
CREATE INDEX idx_audit_logs_actor ON audit_logs (actor_id);
CREATE INDEX idx_audit_logs_target ON audit_logs (target_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs (created_at DESC);
CREATE INDEX idx_audit_logs_guild_created ON audit_logs (guild_id, created_at DESC);
CREATE INDEX idx_audit_logs_guild_type_date ON audit_logs (guild_id, event_type, created_at DESC);

DROP TABLE audit_logs_old;

-- ════════════════════════════════════════════════════════════════════════════
-- 3. USER_ACTIVITY_LOG
-- ════════════════════════════════════════════════════════════════════════════

ALTER TABLE user_activity_log RENAME TO user_activity_log_old;

CREATE TABLE user_activity_log (
    id           UUID NOT NULL DEFAULT gen_random_uuid(),
    guild_id     VARCHAR(20) NOT NULL,
    user_id      VARCHAR(20) NOT NULL,
    event_type   TEXT NOT NULL,
    channel_id   VARCHAR(20),
    channel_name TEXT,
    content      TEXT,
    metadata     JSONB DEFAULT '{}',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE user_activity_log_default PARTITION OF user_activity_log DEFAULT;

CREATE TABLE user_activity_log_2026_04 PARTITION OF user_activity_log FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE user_activity_log_2026_05 PARTITION OF user_activity_log FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE user_activity_log_2026_06 PARTITION OF user_activity_log FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE user_activity_log_2026_07 PARTITION OF user_activity_log FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE user_activity_log_2026_08 PARTITION OF user_activity_log FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE user_activity_log_2026_09 PARTITION OF user_activity_log FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE user_activity_log_2026_10 PARTITION OF user_activity_log FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');
CREATE TABLE user_activity_log_2026_11 PARTITION OF user_activity_log FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');
CREATE TABLE user_activity_log_2026_12 PARTITION OF user_activity_log FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');
CREATE TABLE user_activity_log_2027_01 PARTITION OF user_activity_log FOR VALUES FROM ('2027-01-01') TO ('2027-02-01');
CREATE TABLE user_activity_log_2027_02 PARTITION OF user_activity_log FOR VALUES FROM ('2027-02-01') TO ('2027-03-01');
CREATE TABLE user_activity_log_2027_03 PARTITION OF user_activity_log FOR VALUES FROM ('2027-03-01') TO ('2027-04-01');

INSERT INTO user_activity_log SELECT * FROM user_activity_log_old;

DROP INDEX IF EXISTS idx_user_activity_guild_user;
DROP INDEX IF EXISTS idx_user_activity_created;
DROP INDEX IF EXISTS idx_user_activity_type;
DROP INDEX IF EXISTS idx_user_activity_guild_user_type;

CREATE INDEX idx_user_activity_guild_user ON user_activity_log (guild_id, user_id);
CREATE INDEX idx_user_activity_created ON user_activity_log (created_at);
CREATE INDEX idx_user_activity_type ON user_activity_log (event_type);
CREATE INDEX idx_user_activity_guild_user_type ON user_activity_log (guild_id, user_id, event_type);

DROP TABLE user_activity_log_old;

-- ════════════════════════════════════════════════════════════════════════════
-- 4. LOGS  (cle = `timestamp`, pas `created_at`)
-- ════════════════════════════════════════════════════════════════════════════

ALTER TABLE logs RENAME TO logs_old;

CREATE TABLE logs (
    id        UUID NOT NULL DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    level     VARCHAR(10) NOT NULL DEFAULT 'info',
    bot       VARCHAR(100) NOT NULL DEFAULT '',
    server    VARCHAR(200) NOT NULL DEFAULT '',
    message   TEXT NOT NULL,
    category  VARCHAR(20) NOT NULL DEFAULT 'discord',
    details   JSONB NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (id, timestamp)
) PARTITION BY RANGE (timestamp);

CREATE TABLE logs_default PARTITION OF logs DEFAULT;

CREATE TABLE logs_2026_04 PARTITION OF logs FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE logs_2026_05 PARTITION OF logs FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE logs_2026_06 PARTITION OF logs FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE logs_2026_07 PARTITION OF logs FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE logs_2026_08 PARTITION OF logs FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE logs_2026_09 PARTITION OF logs FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE logs_2026_10 PARTITION OF logs FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');
CREATE TABLE logs_2026_11 PARTITION OF logs FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');
CREATE TABLE logs_2026_12 PARTITION OF logs FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');
CREATE TABLE logs_2027_01 PARTITION OF logs FOR VALUES FROM ('2027-01-01') TO ('2027-02-01');
CREATE TABLE logs_2027_02 PARTITION OF logs FOR VALUES FROM ('2027-02-01') TO ('2027-03-01');
CREATE TABLE logs_2027_03 PARTITION OF logs FOR VALUES FROM ('2027-03-01') TO ('2027-04-01');

INSERT INTO logs SELECT * FROM logs_old;

DROP INDEX IF EXISTS idx_logs_timestamp;
DROP INDEX IF EXISTS idx_logs_level;
DROP INDEX IF EXISTS idx_logs_bot;
DROP INDEX IF EXISTS idx_logs_category;

CREATE INDEX idx_logs_timestamp ON logs (timestamp DESC);
CREATE INDEX idx_logs_level ON logs (level);
CREATE INDEX idx_logs_bot ON logs (bot);

DROP TABLE logs_old;
