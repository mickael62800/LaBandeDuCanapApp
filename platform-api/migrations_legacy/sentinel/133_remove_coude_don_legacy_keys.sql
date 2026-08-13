-- Retire les cles don_* legacy de coude-bot.config_schema.
-- Elles ont ete remplacees par gift_tax_percent / gift_cooldown_secs
-- (migration 131), qui correspondent aux cles reellement lues par le code.
-- Le code n'a jamais lu don_tax_percent ni don_coins_cooldown_secs.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM jsonb_array_elements(config_schema) AS elem
    WHERE elem->>'key' NOT IN ('don_tax_percent', 'don_coins_cooldown_secs')
)
WHERE bot_name = 'coude-bot';

-- Nettoie aussi les valeurs stockees dans bot_guild_config pour ces cles,
-- si certains serveurs les avaient configurees.
DELETE FROM bot_guild_config
WHERE bot_name = 'coude-bot'
  AND config_key IN ('don_tax_percent', 'don_coins_cooldown_secs');
