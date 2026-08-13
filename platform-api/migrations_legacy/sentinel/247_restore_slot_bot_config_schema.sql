-- slot-bot — restauration du schema de jeu.
--
-- Regression : la mig 230 a ecrase entierement le config_schema de slot-bot
-- (INSERT ... ON CONFLICT DO UPDATE SET config_schema = EXCLUDED...) en ne
-- gardant que {enabled, default_bet}. La mig 237 a rajoute slot_category_id.
-- Mais 11 cles encore LUES par le code (sentinel-core/application/casino/
-- manage_slot_service.rs : symbols, weights, payouts, jackpot, bornes de
-- mise, cooldown, daily bonus) ont disparu du schema -> non configurables
-- depuis la page Composants (le jeu tourne sur les defauts hardcodes).
--
-- On les reintegre (defs d'origine, mig 157).

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "min_bet", "label": "Mise min", "type": "number", "required": false, "default": "10", "unit": "coins", "min": 1, "max": 1000000, "description": "Mise minimale par spin.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "max_bet", "label": "Mise max", "type": "number", "required": false, "default": "1000", "unit": "coins", "min": 1, "max": 100000000, "description": "Mise maximale par spin.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "cooldown_secs", "label": "Cooldown entre spins", "type": "number", "required": false, "default": "5", "unit": "secondes", "min": 0, "max": 3600, "description": "Delai entre 2 spins pour un meme joueur. Anti-spam.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "symbols", "label": "Symboles (CSV)", "type": "text", "required": false, "default": "🍒,🍋,🍊,🍇,🔔,⭐,7️⃣", "description": "Liste des symboles separes par virgules. Du plus frequent au plus rare. Le dernier = jackpot.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "weights", "label": "Poids des symboles (CSV)", "type": "text", "required": false, "default": "30,25,20,15,7,2,1", "description": "Poids RNG de chaque symbole (meme ordre que symbols). Plus le poids est grand, plus le symbole sort souvent.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "payout_3x_multipliers", "label": "Multiplicateurs 3 identiques (CSV)", "type": "text", "required": false, "default": "2,3,5,8,12,25,100", "description": "Multiplicateur de la mise pour 3 identiques (meme ordre que symbols). Le dernier = jackpot.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "payout_2x_enabled", "label": "Payout sur 2 identiques", "type": "boolean", "required": false, "default": "true", "description": "Si ON, 2 symboles identiques sur 3 = remboursement de la mise (1x).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "jackpot_pool_share_pct", "label": "% mise vers jackpot", "type": "number", "required": false, "default": "1", "unit": "%", "min": 0, "max": 50, "description": "Pourcentage de chaque mise qui alimente le pool jackpot progressif. Recommande : 1-5%.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "jackpot_starting_pool", "label": "Pool jackpot de depart", "type": "number", "required": false, "default": "1000", "unit": "coins", "min": 0, "max": 100000000, "description": "Valeur de depart du pool jackpot (et reset a chaque jackpot decroche).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "daily_bonus_enabled", "label": "Daily bonus actif", "type": "boolean", "required": false, "default": "true", "description": "Si ON, chaque joueur peut faire 1 spin gratuit par jour.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "daily_bonus_mise", "label": "Mise du spin gratuit", "type": "number", "required": false, "default": "100", "unit": "coins", "min": 1, "max": 1000000, "description": "Mise utilisee pour le spin gratuit quotidien (le payout suit cette mise).", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'slot-bot'
  AND NOT (config_schema @> '[{"key": "symbols"}]'::jsonb);
