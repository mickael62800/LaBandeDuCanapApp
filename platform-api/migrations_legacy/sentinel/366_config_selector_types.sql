-- Reglages de salons/categories/roles : le SCHEMA devient auto-descriptif.
--
-- Jusqu'ici, certains champs etaient declares "text" (liste d'IDs) ou "channel"
-- (alors qu'il fallait un salon VOCAL), et c'est le front qui devinait le bon
-- selecteur via des listes de cles codees en dur. Resultat : une nouvelle cle
-- n'avait le bon widget que si quelqu'un pensait a editer le Vue.
--
-- On introduit des types de premier ordre :
--   voice        -> selecteur de salon VOCAL (unique)
--   channel_list -> multi-selecteur de salons textuels
--   voice_list   -> multi-selecteur de salons vocaux
--   role_list    -> multi-selecteur de roles
--
-- Le front garde les anciennes listes de cles en repli, donc cette migration
-- est sure meme si le deploiement web n'est pas encore a jour.

-- 1. Salons VOCAUX uniques : etaient "channel" (dropdown de salons textuels).
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE
            WHEN f->>'key' IN (
                'public_creator_channel_id',
                'private_creator_channel_id',
                'game_creator_channel_id',
                'afk_channel_id',
                'counter_channel_id',
                'voice_counter_channel_id'
            ) AND f->>'type' = 'channel'
            THEN jsonb_set(f, '{type}', '"voice"')
            ELSE f
        END
        ORDER BY idx
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(f, idx)
)
WHERE config_schema @> '[{"type":"channel"}]'::jsonb;

-- 2. Listes d'IDs : etaient "text" (saisie libre / CSV).
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE
            WHEN f->>'type' <> 'text' THEN f
            WHEN f->>'key' = 'observed_voice_channels' THEN jsonb_set(f, '{type}', '"voice_list"')
            WHEN f->>'key' IN (
                'ignored_channels',
                'excluded_channels',
                'whitelist_channels',
                'exempt_channels',
                'command_channels'
            ) THEN jsonb_set(f, '{type}', '"channel_list"')
            WHEN f->>'key' IN (
                'ignored_roles',
                'excluded_roles',
                'whitelist_roles',
                'exempt_roles',
                'double_xp_roles',
                'monthly_ranking_excluded_roles',
                'rules_role_id'
            ) THEN jsonb_set(f, '{type}', '"role_list"')
            ELSE f
        END
        ORDER BY idx
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(f, idx)
)
WHERE config_schema @> '[{"type":"text"}]'::jsonb;
