-- Phase 2 A.3 — Breaking changes (partie 1) : enums Postgres + BIGINT permissions
--
-- Note : on EXCLUT volontairement de cette migration :
--   - bot_guild_config.config_value TEXT -> JSONB (23 callsites bots, defere)
--   - infractions.action TEXT -> ENUM (deja gere par enum Rust cote applicatif)
--   - NOT NULL/CHECK constraints (risque sur donnees existantes, defere)
--
-- Les 3 enums + le BIGINT permissions sont consolides ici car ils touchent
-- des fichiers Rust differents et limites (3 entities, 3 repositories, 2 DTOs).

-- ── Enums Postgres ───────────────────────────────────────────────────────────

-- Coude classes : 4 valeurs canoniques (bots/coude-bot/src/game/classes.rs)
DO $$ BEGIN
    CREATE TYPE coude_class AS ENUM ('bourrin', 'agile', 'fourbe', 'tank');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- Gravite des actions de moderation
DO $$ BEGIN
    CREATE TYPE moderation_gravity AS ENUM ('low', 'medium', 'high', 'critical');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- Type de salon vocal temporaire
DO $$ BEGIN
    CREATE TYPE voice_channel_kind AS ENUM ('public', 'private');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- ── ALTER COLUMN avec cast USING ─────────────────────────────────────────────
-- Le cast `column::enum_name` echoue si une valeur n'est pas dans l'enum :
-- normaliser les donnees existantes d'abord si necessaire.

-- coude_players.class : la migration 102 (mv_coude_leaderboard) lit ce champ,
-- on doit DROP la MV avant le ALTER puis la recreer apres.
DROP MATERIALIZED VIEW IF EXISTS mv_coude_leaderboard;

-- Normaliser les valeurs hors enum (defensif)
UPDATE coude_players SET class = 'bourrin'
  WHERE class IS NOT NULL AND class NOT IN ('bourrin', 'agile', 'fourbe', 'tank');

-- DROP le DEFAULT avant le ALTER TYPE : PostgreSQL ne peut pas caster un
-- DEFAULT TEXT ('bourrin'::text) vers un type ENUM automatiquement.
ALTER TABLE coude_players ALTER COLUMN class DROP DEFAULT;

ALTER TABLE coude_players
    ALTER COLUMN class TYPE coude_class USING class::coude_class;

-- Re-etablir le DEFAULT avec le nouveau type
ALTER TABLE coude_players ALTER COLUMN class SET DEFAULT 'bourrin'::coude_class;

-- Recreer la MV (identique a 102)
CREATE MATERIALIZED VIEW mv_coude_leaderboard AS
SELECT
    guild_id, user_id, username, coins,
    total_wins, total_losses, total_draws,
    total_earned, total_lost, total_stolen,
    cowardice_count, chaos_events, casino_wins, casino_losses,
    level, xp, stat_points, atk, def, class, title,
    hp_current, hp_max, hp_last_regen, repos_last_used, class_changed_at,
    season, created_at, updated_at,
    ROW_NUMBER() OVER (PARTITION BY guild_id ORDER BY coins DESC) AS rank
FROM coude_players;

CREATE UNIQUE INDEX uq_mv_coude_leaderboard
    ON mv_coude_leaderboard (guild_id, user_id);
CREATE INDEX idx_mv_coude_leaderboard_rank
    ON mv_coude_leaderboard (guild_id, rank);

-- moderation_actions.gravity : peut etre NULL, normaliser les valeurs hors enum
UPDATE moderation_actions SET gravity = NULL
  WHERE gravity IS NOT NULL AND gravity NOT IN ('low', 'medium', 'high', 'critical');

ALTER TABLE moderation_actions
    ALTER COLUMN gravity TYPE moderation_gravity USING gravity::moderation_gravity;

-- voice_channels.kind : NOT NULL, normaliser
UPDATE voice_channels SET kind = 'public'
  WHERE kind NOT IN ('public', 'private');

-- DROP le DEFAULT avant le ALTER TYPE (meme raison que coude_players.class)
ALTER TABLE voice_channels ALTER COLUMN kind DROP DEFAULT;

ALTER TABLE voice_channels
    ALTER COLUMN kind TYPE voice_channel_kind USING kind::voice_channel_kind;

ALTER TABLE voice_channels ALTER COLUMN kind SET DEFAULT 'public'::voice_channel_kind;

-- ── discord_roles.permissions TEXT -> BIGINT ─────────────────────────────────
-- Discord permissions sont des bitfields 64 bits. Stocker en BIGINT permet
-- les operations bitwise SQL (`permissions & ADMINISTRATOR_BIT`) sans cast.
-- Cote API on continue de serialiser en String dans le DTO pour la safety
-- JS (BigInt > Number.MAX_SAFE_INTEGER pour certains bits futurs).

-- Defensive : remplacer les valeurs vides ou non-numeriques par '0'
UPDATE discord_roles SET permissions = '0'
  WHERE permissions !~ '^[0-9]+$';

-- DROP le DEFAULT TEXT avant le ALTER TYPE BIGINT (meme pattern que les enums)
ALTER TABLE discord_roles ALTER COLUMN permissions DROP DEFAULT;

ALTER TABLE discord_roles
    ALTER COLUMN permissions TYPE BIGINT USING permissions::bigint;

ALTER TABLE discord_roles
    ALTER COLUMN permissions SET DEFAULT 0;
