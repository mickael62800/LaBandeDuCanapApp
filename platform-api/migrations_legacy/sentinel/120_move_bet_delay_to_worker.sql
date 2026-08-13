-- Deplace la cle `bet_delay_secs` (duree de la phase de paris) du schema
-- `coude-bot` vers `coude-worker`.
--
-- Motivation : c'est le coude-worker qui lit effectivement cette valeur
-- pour decider quand resoudre un combat en phase betting. La mettre dans
-- le schema coude-bot induit l'utilisateur en erreur (il edite dans la
-- mauvaise page de l'UI desktop et la valeur est jamais prise en compte).
--
-- Trois etapes :
--   1) Append au schema coude-worker
--   2) Retire la cle du schema coude-bot via jsonb_agg avec filtre
--   3) Migre les valeurs existantes : toute ligne `bot_guild_config` avec
--      bot_name IN ('coude','coude-bot') et config_key = 'bet_delay_secs'
--      est reaffectee a bot_name = 'coude-worker' (ou supprimee si doublon).

-- 1) Ajout au schema coude-worker
UPDATE bot_definitions
SET config_schema = config_schema::jsonb || '[
    {
        "key": "bet_delay_secs",
        "label": "Delai paris (secondes)",
        "type": "number",
        "required": false,
        "default": "300",
        "description": "Duree de la phase de paris apres acceptation d''un defi, avant que le combat soit resolu. Lu par le coude-worker."
    }
]'::jsonb
WHERE bot_name = 'coude-worker';

-- 2) Retrait de la cle du schema coude-bot
UPDATE bot_definitions
SET config_schema = COALESCE((
    SELECT jsonb_agg(elem)
    FROM jsonb_array_elements(config_schema::jsonb) elem
    WHERE elem->>'key' <> 'bet_delay_secs'
), '[]'::jsonb)::jsonb
WHERE bot_name = 'coude-bot';

-- 3) Migration des valeurs existantes
--    a) Supprimer les lignes coude-worker pre-existantes pour eviter
--       le conflit de cle unique lors de l'UPDATE qui suit.
DELETE FROM bot_guild_config
WHERE bot_name = 'coude-worker'
  AND config_key = 'bet_delay_secs'
  AND guild_id IN (
      SELECT guild_id FROM bot_guild_config
      WHERE bot_name IN ('coude', 'coude-bot')
        AND config_key = 'bet_delay_secs'
  );

--    b) Reaffecter les anciennes lignes
UPDATE bot_guild_config
SET bot_name = 'coude-worker'
WHERE bot_name IN ('coude', 'coude-bot')
  AND config_key = 'bet_delay_secs';
