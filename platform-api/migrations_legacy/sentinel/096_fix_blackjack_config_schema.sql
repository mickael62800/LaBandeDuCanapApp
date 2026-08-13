-- Fix blackjack-bot config_schema : format tableau (comme automod-bot) + parametres manquants
UPDATE bot_definitions SET config_schema = '[
    {"key": "enabled", "label": "Activer le Blackjack", "type": "boolean", "required": false, "default": "true"},

    {"key": "channel_blackjack", "label": "Salon Blackjack (panel)", "type": "channel", "required": false},
    {"key": "category_blackjack", "label": "Categorie pour les tables privees", "type": "channel", "required": false},
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},

    {"key": "min_bet", "label": "Mise minimale", "type": "number", "required": false, "default": "10"},
    {"key": "max_bet", "label": "Mise maximale (0 = illimite)", "type": "number", "required": false, "default": "1000"},
    {"key": "starting_coins", "label": "Coins de depart (nouveaux joueurs)", "type": "number", "required": false, "default": "200"},
    {"key": "blackjack_payout", "label": "Multiplicateur blackjack naturel (ex: 1.5 = x1.5)", "type": "number", "required": false, "default": "1.5"},

    {"key": "cooldown_secs", "label": "Cooldown entre parties (secondes)", "type": "number", "required": false, "default": "30"},
    {"key": "max_daily_games", "label": "Parties max par jour (0 = illimite)", "type": "number", "required": false, "default": "0"},
    {"key": "afk_timeout_secs", "label": "Timeout AFK table (secondes)", "type": "number", "required": false, "default": "1800"},

    {"key": "allow_double_down", "label": "Autoriser le Doubler", "type": "boolean", "required": false, "default": "true"},
    {"key": "max_players_per_table", "label": "Joueurs max par table", "type": "number", "required": false, "default": "5"},
    {"key": "shoe_decks", "label": "Nombre de decks dans le sabot", "type": "number", "required": false, "default": "6"}
]' WHERE bot_name = 'blackjack-bot';
