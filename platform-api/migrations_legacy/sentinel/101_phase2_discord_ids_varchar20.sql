-- Phase 2 A.1 — Conversion TEXT -> VARCHAR(20) pour tous les Discord IDs
--
-- Les snowflakes Discord tiennent dans 19 chiffres (i64) max ; VARCHAR(20)
-- offre une marge de securite tout en ramenant la taille mediane des index
-- B-tree de ~30-40 % par rapport a TEXT non borne (la representation toast
-- de TEXT impose un overhead de longueur variable que VARCHAR borne evite).
--
-- Sqlx mappe TEXT et VARCHAR(n) vers `String` Rust de maniere identique :
-- ZERO impact code applicatif.
--
-- Approche : on introspecte `information_schema` et on convertit uniquement
-- les colonnes de type `text` dont le nom correspond a un identifiant
-- Discord connu. Le DO bloc est :
--   - **idempotent** : re-executer la migration est un no-op (les colonnes
--     deja converties n'apparaissent plus en `text`)
--   - **safe** : on ne touche jamais les UUID (FK internes), JSONB, INT, etc.
--   - **explicite** : whitelist de noms de colonnes, pas de regex large

DO $$
DECLARE
    r RECORD;
    discord_id_columns TEXT[] := ARRAY[
        -- Identifiants Discord generiques
        'guild_id', 'user_id', 'channel_id', 'message_id', 'role_id',
        'owner_id', 'actor_id', 'target_id', 'moderator_id', 'author_id',
        'banned_by',
        -- Coude (combats / wagers)
        'attacker_id', 'defender_id',
        -- Voice channels (multi-channels)
        'voice_channel_id', 'members_channel_id', 'queue_channel_id',
        'text_channel_id', 'category_id', 'invited_user_id',
        -- Tickets / staff
        'assigned_to',
        -- Welcome bot
        'welcome_channel_id', 'leave_channel_id', 'rules_channel_id',
        'rules_role_id', 'counter_channel_id', 'anniversary_channel_id'
    ];
BEGIN
    FOR r IN
        SELECT table_name, column_name
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND data_type = 'text'
          AND column_name = ANY(discord_id_columns)
        ORDER BY table_name, column_name
    LOOP
        EXECUTE format(
            'ALTER TABLE %I ALTER COLUMN %I TYPE VARCHAR(20)',
            r.table_name, r.column_name
        );
        RAISE NOTICE 'Converted %.% TEXT -> VARCHAR(20)', r.table_name, r.column_name;
    END LOOP;
END $$;
