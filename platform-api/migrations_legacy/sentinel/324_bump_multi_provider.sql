-- ============================================
-- Bump multi-provider : le systeme de bump n'est plus lie a Disboard seul.
-- On ajoute une colonne `provider` a `bump_events` et `bump_guild_state`, et on
-- fait passer l'etat de rappel a une cle (guild_id, provider) : chaque
-- plateforme (Disboard, DiscordL, ...) suit son propre cooldown/rappel.
--
-- Retrocompatible : les lignes existantes sont backfillees a 'disboard' via le
-- DEFAULT. Idempotente : rejouable sans effet de bord.
-- ============================================

-- 1) bump_events : provenance du bump.
ALTER TABLE bump_events
    ADD COLUMN IF NOT EXISTS provider TEXT NOT NULL DEFAULT 'disboard';

-- 2) bump_guild_state : provider + PK (guild_id, provider).
ALTER TABLE bump_guild_state
    ADD COLUMN IF NOT EXISTS provider TEXT NOT NULL DEFAULT 'disboard';

-- Backfill explicite (le DEFAULT couvre deja les lignes existantes ; ceinture
-- et bretelles au cas ou la colonne aurait ete ajoutee sans defaut).
UPDATE bump_guild_state SET provider = 'disboard' WHERE provider IS NULL OR provider = '';

-- Remplace la PK (guild_id) par (guild_id, provider) si ce n'est pas deja le
-- cas. On identifie la PK par ensemble de colonnes (pas par nom).
DO $$
DECLARE
    guild_attnum    smallint;
    provider_attnum smallint;
    pk_name         text;
    pk_is_correct   boolean;
BEGIN
    SELECT attnum INTO guild_attnum
    FROM pg_attribute
    WHERE attrelid = 'bump_guild_state'::regclass AND attname = 'guild_id';

    SELECT attnum INTO provider_attnum
    FROM pg_attribute
    WHERE attrelid = 'bump_guild_state'::regclass AND attname = 'provider';

    -- PK existante deja exactement sur (guild_id, provider) ?
    SELECT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'bump_guild_state'::regclass
          AND contype = 'p'
          AND conkey @> ARRAY[guild_attnum, provider_attnum]
          AND conkey <@ ARRAY[guild_attnum, provider_attnum]
    ) INTO pk_is_correct;

    IF NOT pk_is_correct THEN
        -- Supprime toute PK existante sur la table (quel que soit son nom).
        SELECT conname INTO pk_name
        FROM pg_constraint
        WHERE conrelid = 'bump_guild_state'::regclass AND contype = 'p'
        LIMIT 1;

        IF pk_name IS NOT NULL THEN
            EXECUTE format('ALTER TABLE bump_guild_state DROP CONSTRAINT %I', pk_name);
        END IF;

        ALTER TABLE bump_guild_state
            ADD CONSTRAINT bump_guild_state_pkey PRIMARY KEY (guild_id, provider);
    END IF;
END $$;

-- 3) Schema de config du dashboard : cles multi-provider (idempotent via @>).
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "disboard_enabled", "label": "Provider Disboard actif", "type": "boolean", "required": false, "default": "true", "description": "Recompense les bumps Disboard (necessite aussi le module actif).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "discordl_enabled", "label": "Provider DiscordL actif", "type": "boolean", "required": false, "default": "true", "description": "Recompense les bumps DiscordL (discordl.org) (necessite aussi le module actif).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "discordl_cooldown_minutes", "label": "Cooldown DiscordL (minutes)", "type": "number", "required": false, "default": "240", "description": "Delai DiscordL entre deux bumps (defaut 240 = 4h).", "depends_on": {"key": "discordl_enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'bump-bot'
  AND NOT (config_schema @> '[{"key": "discordl_enabled"}]'::jsonb);
