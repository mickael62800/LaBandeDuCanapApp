-- Jeu « Influence » — l'« Argent » utilise desormais le WALLET PARTAGE
-- (user_wallets.coins), la meme monnaie que Coup de Coude / casino, au lieu
-- d'une monnaie parallele. On retire donc le reglage trompeur
-- `influence_start_money` (le solde de depart vient deja du systeme de coins).

UPDATE bot_definitions
SET config_schema = (
    SELECT COALESCE(jsonb_agg(elem), '[]'::jsonb)
    FROM jsonb_array_elements(config_schema) AS elem
    WHERE elem->>'key' <> 'influence_start_money'
)
WHERE bot_name = 'influence-bot';
