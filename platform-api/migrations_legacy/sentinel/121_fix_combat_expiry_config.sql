-- Nettoie le bordel autour de "duree d'expiration d'un combat".
--
-- Etat avant :
--   - Schema coude-bot    : cle `combat_expire_secs` (secondes, defaut 86400)
--     ajoutee par la migration 078, mais JAMAIS lue par le bot.
--   - Schema coude-worker : cle `combat_expiry_hours` (heures, defaut 24)
--     seedee par 061, visible dans l'UI worker mais JAMAIS lue nulle part.
--   - Code expire_combats : lit `combat_expire_secs` avec `bot_name = 'coude'`
--     qui n'existe dans AUCUN schema — donc la valeur editee depuis l'UI
--     n'etait jamais prise en compte et le worker tournait toujours sur
--     86400 s en dur.
--
-- On rationnalise :
--   - On garde SEULEMENT `combat_expiry_hours` sur `coude-worker` (source
--     de verite, visible dans la page de config Worker Coup de Coude).
--   - On retire `combat_expire_secs` du schema coude-bot.
--   - On migre les valeurs existantes : si une guild avait
--     `bot_name IN ('coude','coude-bot')`, `config_key = 'combat_expire_secs'`,
--     on la reaffecte a `bot_name = 'coude-worker'`, `config_key = 'combat_expiry_hours'`
--     en convertissant les secondes en heures (arrondi superieur, min 1).

-- 1) Retirer combat_expire_secs du schema coude-bot
UPDATE bot_definitions
SET config_schema = COALESCE((
    SELECT jsonb_agg(elem)
    FROM jsonb_array_elements(config_schema::jsonb) elem
    WHERE elem->>'key' <> 'combat_expire_secs'
), '[]'::jsonb)::jsonb
WHERE bot_name = 'coude-bot';

-- 2) Migration des valeurs existantes
--    a) Supprimer d'abord les lignes coude-worker/combat_expiry_hours pour
--       ces guilds afin d'eviter un conflit de cle unique a l'etape b).
DELETE FROM bot_guild_config
WHERE bot_name = 'coude-worker'
  AND config_key = 'combat_expiry_hours'
  AND guild_id IN (
      SELECT guild_id FROM bot_guild_config
      WHERE bot_name IN ('coude', 'coude-bot')
        AND config_key = 'combat_expire_secs'
  );

--    b) Reinserer en convertissant secondes -> heures (arrondi superieur,
--       minimum 1 heure).
INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
SELECT
    guild_id,
    'coude-worker',
    'combat_expiry_hours',
    GREATEST(
        1,
        CEIL(
            (CASE WHEN config_value ~ '^\d+$' THEN config_value::int ELSE 86400 END)::float
            / 3600.0
        )::int
    )::text
FROM bot_guild_config
WHERE bot_name IN ('coude', 'coude-bot')
  AND config_key = 'combat_expire_secs';

--    c) Supprimer les anciennes lignes
DELETE FROM bot_guild_config
WHERE bot_name IN ('coude', 'coude-bot')
  AND config_key = 'combat_expire_secs';
