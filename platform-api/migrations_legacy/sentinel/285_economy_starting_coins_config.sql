-- Solde de depart des joueurs, configurable par serveur depuis le web
-- (remplace la variable d'env globale WALLET_STARTING_COINS, qui reste un
-- fallback). Append non destructif au config_schema coude-bot.
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "starting_coins", "label": "Solde de depart", "type": "number", "required": false, "default": "100", "min": 0, "max": 1000000000, "unit": "coins", "description": "Coins offerts a la creation du portefeuille d un nouveau joueur."}
]'::jsonb
WHERE bot_name = 'coude-bot';
