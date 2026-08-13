-- Phase 2 A.1 — Quick wins zero-breaking (cf. docs/ROADMAP.md & docs/DB_OPTIMISATIONS.md)
--
-- Objectifs :
--   1. Supprimer les index simples redondants (couverts par des composites)
--   2. Convertir les index "soft-delete" en index partiels (5-10x plus petits)
--   3. Ajouter des index GIN sur les colonnes JSONB frequemment requetees
--   4. Supprimer les colonnes mortes residuelles
--
-- Aucun impact code applicatif : sqlx ne voit aucune difference.
-- Gain attendu : -20 a -30 % taille index totale, +10-50x sur queries JSONB,
-- +5-10x sur queries WHERE status='open' grace aux partials.

-- ── 1. Index simples redondants (prefixes de composites) ─────────────────────

-- idx_audit_logs_guild (guild_id) est un prefixe strict de
-- idx_audit_logs_guild_created (guild_id, created_at DESC), donc Postgres
-- utilisera toujours le composite. L'index simple est mort.
DROP INDEX IF EXISTS idx_audit_logs_guild;

-- idx_infractions_guild (guild_id) est couvert par plusieurs composites :
-- idx_infractions_user (guild_id, user_id), idx_infractions_guild_created
-- (guild_id, created_at DESC), idx_infractions_guild_action (guild_id, action),
-- idx_infractions_guild_action_created (guild_id, action, created_at DESC).
DROP INDEX IF EXISTS idx_infractions_guild;

-- ── 2. Index partiels sur les tables soft-delete ─────────────────────────────

-- voice_channels : 99 % des queries filtrent sur les salons "open".
-- L'ancien idx_voice_channels_status indexait toutes les lignes (open + closed)
-- alors que le hot path ne touche que 'open'. On remplace par un partiel
-- couvrant directement les colonnes du WHERE typique (guild_id + owner_id).
DROP INDEX IF EXISTS idx_voice_channels_status;
CREATE INDEX IF NOT EXISTS idx_voice_channels_active
    ON voice_channels (guild_id, owner_id)
    WHERE channel_status = 'open';

-- tickets : meme logique. Les tickets fermes/archives ne sont presque jamais
-- listes (UI affiche d'abord les ouverts). Note : tickets n'a pas de guild_id,
-- on indexe sur (server, created_at DESC) pour le tri chronologique du listing.
DROP INDEX IF EXISTS idx_tickets_status;
CREATE INDEX IF NOT EXISTS idx_tickets_open
    ON tickets (server, created_at DESC)
    WHERE status IN ('open', 'assigned');

-- ── 3. Index GIN sur les colonnes JSONB frequemment requetees ────────────────

-- infractions.flags : JSONB stockant les categories detectees par l'automod
-- (toxicity, spam, link, etc.). Queries analytics du dashboard utilisent
-- des operateurs @> et ? pour filtrer sur certaines categories.
CREATE INDEX IF NOT EXISTS idx_infractions_flags_gin
    ON infractions USING GIN (flags);

-- security_events.user_ids : array JSON des users impliques dans un evenement
-- (raid, mass-mention, etc.). Queries "events impliquant user X" sont O(N)
-- sans GIN.
CREATE INDEX IF NOT EXISTS idx_security_events_user_ids_gin
    ON security_events USING GIN (user_ids);

-- bot_definitions.config_schema : JSONB du schema de config UI (cf. dashboard
-- desktop qui le charge a chaque ouverture de page bot). Petite table mais
-- requetee a chaque navigation.
CREATE INDEX IF NOT EXISTS idx_bot_definitions_config_schema_gin
    ON bot_definitions USING GIN (config_schema);

-- ── 4. Colonnes mortes (audit code : 0 reference Rust + frontend) ────────────

-- coude_combats.channel_id_temp : ajoutee en 082 lors du refactor v2 HP system,
-- jamais branchee, jamais lue par aucun handler / repository. Safe a drop.
ALTER TABLE coude_combats DROP COLUMN IF EXISTS channel_id_temp;
