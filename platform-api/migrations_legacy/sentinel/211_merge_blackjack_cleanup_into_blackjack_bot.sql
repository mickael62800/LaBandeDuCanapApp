-- Phase composants — Fusion `blackjack-cleanup-worker` dans `blackjack-bot`.
--
-- blackjack-bot : 13 cles metier (mises, payout, AFK timeout, decks).
-- blackjack-cleanup-worker : 1 cle infra (blackjack_cleanup_scan_interval).
--
-- Schema fusionne avec depends_on en cascade :
--   enabled
--   ├─ channel_blackjack, category_blackjack, log_channel_id
--   ├─ min_bet, max_bet, starting_coins, blackjack_payout
--   ├─ cooldown_secs, max_daily_games
--   ├─ afk_timeout_secs
--   │   └─ blackjack_cleanup_scan_interval (depend du timeout AFK,
--   │       car sans timeout AFK le scan periodique n'a aucun sens)
--   ├─ allow_double_down
--   ├─ max_players_per_table
--   └─ shoe_decks

-- 1) Restaure le bot_name='blackjack-bot' (mig 204 avait renomme
-- blackjack-cleanup-worker -> blackjack).
DELETE FROM bot_guild_config wkr
    WHERE wkr.bot_name = 'blackjack'
      AND EXISTS (
          SELECT 1 FROM bot_guild_config m
           WHERE m.bot_name = 'blackjack-bot'
             AND m.guild_id = wkr.guild_id
             AND m.config_key = wkr.config_key
      );
UPDATE bot_guild_config SET bot_name = 'blackjack-bot'
    WHERE bot_name = 'blackjack';

-- 2) Schema fusionne avec cascade depends_on.
UPDATE bot_definitions SET
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active le mini-jeu Blackjack."},
        {"key": "channel_blackjack", "label": "Salon Blackjack (panel)", "type": "channel", "required": false, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "category_blackjack", "label": "Categorie pour les tables privees", "type": "channel", "required": false, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false, "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "min_bet", "label": "Mise minimale", "type": "number", "required": false, "default": "10", "min": 1, "max": 1000000, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "max_bet", "label": "Mise maximale (0 = illimite)", "type": "number", "required": false, "default": "1000", "min": 0, "max": 100000000, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "starting_coins", "label": "Coins de depart (nouveaux joueurs)", "type": "number", "required": false, "default": "200", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "blackjack_payout", "label": "Multiplicateur blackjack naturel", "type": "number", "required": false, "default": "1.5", "description": "x1.5 par defaut. Mettre 1.0 pour neutraliser l avantage joueur.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "cooldown_secs", "label": "Cooldown entre parties", "type": "number", "required": false, "default": "30", "min": 0, "max": 3600, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "max_daily_games", "label": "Parties max par jour (0 = illimite)", "type": "number", "required": false, "default": "0", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "afk_timeout_secs", "label": "Timeout AFK table", "type": "number", "required": false, "default": "1800", "min": 30, "max": 7200, "unit": "s", "description": "Apres ce delai sans action, le joueur est eject de la table et la table fermee.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "blackjack_cleanup_scan_interval", "label": "Worker : intervalle scan AFK", "type": "number", "required": false, "default": "60", "min": 10, "max": 600, "unit": "s", "description": "Frequence du scan worker pour ejecter les joueurs AFK et fermer les tables expirees. Doit etre << afk_timeout_secs.", "depends_on": {"key": "afk_timeout_secs", "equals": ""}},

        {"key": "allow_double_down", "label": "Autoriser le Doubler", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "max_players_per_table", "label": "Joueurs max par table", "type": "number", "required": false, "default": "5", "min": 1, "max": 12, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "shoe_decks", "label": "Nombre de decks dans le sabot", "type": "number", "required": false, "default": "6", "min": 1, "max": 12, "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'blackjack-bot';

-- 3) Supprime la definition worker.
DELETE FROM bot_definitions WHERE bot_name = 'blackjack-cleanup-worker';
