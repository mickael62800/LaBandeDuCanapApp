-- Definitions du bot et worker Coup de Coude avec tous les parametres configurables.

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema) VALUES
('coude-bot', 'Bot Coup de Coude', 'Mini-jeu social chaotique : combats, paris, vol, casino, primes', '[
  {"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"},
  {"key": "starting_coins", "label": "Coins de depart", "type": "number", "required": false, "default": "200"},
  {"key": "min_bet", "label": "Mise minimum", "type": "number", "required": false, "default": "1"},
  {"key": "max_bet", "label": "Mise maximum (0 = illimite)", "type": "number", "required": false, "default": "0"},
  {"key": "default_bet", "label": "Mise par defaut", "type": "number", "required": false, "default": "10"},
  {"key": "chaos_enabled", "label": "Evenements chaos actifs", "type": "boolean", "required": false, "default": "true"},
  {"key": "chaos_chance", "label": "Chance de chaos (%)", "type": "number", "required": false, "default": "18"},
  {"key": "casino_enabled", "label": "Casino actif", "type": "boolean", "required": false, "default": "true"},
  {"key": "casino_max_bet", "label": "Mise max casino", "type": "number", "required": false, "default": "500"},
  {"key": "steal_enabled", "label": "Vol actif", "type": "boolean", "required": false, "default": "true"},
  {"key": "steal_success_rate", "label": "Taux reussite vol (%)", "type": "number", "required": false, "default": "30"},
  {"key": "steal_cooldown_secs", "label": "Cooldown vol (secondes)", "type": "number", "required": false, "default": "1800"},
  {"key": "insurance_cost", "label": "Cout assurance", "type": "number", "required": false, "default": "50"},
  {"key": "insurance_duration_secs", "label": "Duree assurance (secondes)", "type": "number", "required": false, "default": "3600"},
  {"key": "insurance_scam_rate", "label": "Taux arnaque assurance (%)", "type": "number", "required": false, "default": "5"},
  {"key": "cowardice_threshold", "label": "Seuil lachete (role poule)", "type": "number", "required": false, "default": "5"},
  {"key": "cowardice_penalty", "label": "Penalite lachete sur gains (%)", "type": "number", "required": false, "default": "20"},
  {"key": "refusal_penalty", "label": "Penalite refus (% de la mise)", "type": "number", "required": false, "default": "20"},
  {"key": "daily_chaos_enabled", "label": "Chaos quotidien actif", "type": "boolean", "required": false, "default": "true"},
  {"key": "daily_chaos_percent", "label": "Chaos quotidien : % vole", "type": "number", "required": false, "default": "20"},
  {"key": "happy_hour_multiplier", "label": "Multiplicateur Happy Hour", "type": "number", "required": false, "default": "2"},
  {"key": "log_channel_id", "label": "Salon de logs", "type": "text", "required": false, "default": ""},
  {"key": "shop_explosion_price", "label": "Prix Explosion", "type": "number", "required": false, "default": "200"},
  {"key": "shop_inversion_price", "label": "Prix Inversion", "type": "number", "required": false, "default": "500"},
  {"key": "shop_mindgame_price", "label": "Prix Mindgame", "type": "number", "required": false, "default": "150"},
  {"key": "shop_rage_price", "label": "Prix Rage", "type": "number", "required": false, "default": "100"},
  {"key": "shop_surprise_price", "label": "Prix Attaque Surprise", "type": "number", "required": false, "default": "300"},
  {"key": "shop_double_coup_price", "label": "Prix Double Coup", "type": "number", "required": false, "default": "250"},
  {"key": "shop_coup_traitre_price", "label": "Prix Coup Traitre", "type": "number", "required": false, "default": "350"}
]'::jsonb),
('coude-worker', 'Worker Coup de Coude', 'Expiration des combats en attente et nettoyage', '[
  {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true"},
  {"key": "combat_expiry_check_secs", "label": "Intervalle verification (secondes)", "type": "number", "required": false, "default": "86400"},
  {"key": "combat_expiry_hours", "label": "Delai expiration combat (heures)", "type": "number", "required": false, "default": "24"},
  {"key": "expiry_penalty_percent", "label": "Penalite expiration (% de la mise)", "type": "number", "required": false, "default": "20"},
  {"key": "refund_bets_on_expiry", "label": "Rembourser les paris a l expiration", "type": "boolean", "required": false, "default": "true"}
]'::jsonb)
ON CONFLICT (bot_name) DO UPDATE SET
  display_name = EXCLUDED.display_name,
  description = EXCLUDED.description,
  config_schema = EXCLUDED.config_schema;
