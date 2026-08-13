-- Phase composants — Fusion du worker `coude-worker` dans le module
-- `coude-bot`.
--
-- Schema fusionne : ~33 cles total (28 metier + 5 worker), organisees
-- en hierarchie depends_on pour griser les sous-options non
-- pertinentes :
--
--   enabled (toggle principal)
--   ├─ chaos_enabled
--   │   ├─ chaos_chance
--   │   └─ daily_chaos_enabled
--   │       └─ daily_chaos_percent
--   ├─ casino_enabled
--   │   └─ casino_max_bet
--   ├─ steal_enabled
--   │   ├─ steal_success_rate
--   │   └─ steal_cooldown_secs
--   └─ ... toutes les autres cles depends_on enabled
--
-- Note : la migration 204 avait renomme les rows worker de
-- 'coude-worker' vers 'coude'. On les remet sous 'coude-bot' pour
-- cohérence (le code Rust de coude utilise 'coude-bot'). Il faut
-- aussi mettre a jour WORKER_MODULES cote sentinel-worker.

-- 1) Restaure les configs worker sous le bot_name du module
-- (annule partiellement la migration 204 pour le seul cas coude).
-- Supprime d'abord les doublons (cle worker == cle deja existante cote
-- module, typiquement 'enabled') pour eviter une violation d'unicite.
DELETE FROM bot_guild_config wkr
    WHERE wkr.bot_name = 'coude'
      AND EXISTS (
          SELECT 1 FROM bot_guild_config m
           WHERE m.bot_name = 'coude-bot'
             AND m.guild_id = wkr.guild_id
             AND m.config_key = wkr.config_key
      );
UPDATE bot_guild_config SET bot_name = 'coude-bot'
    WHERE bot_name = 'coude';

-- 2) Schema fusionne avec depends_on pour la cascade UI.
UPDATE bot_definitions SET
    display_name = 'Coup de Coude',
    description = 'Mini-jeu social chaotique : combats, paris, vol, casino, chaos quotidien. Le timer d''expiration des combats tourne dans sentinel-worker.',
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active toutes les fonctionnalites Coup de Coude pour ce serveur."},
        {"key": "starting_coins", "label": "Coins de depart", "type": "number", "required": false, "default": "200", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "min_bet", "label": "Mise minimum", "type": "number", "required": false, "default": "1", "min": 1, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "max_bet", "label": "Mise maximum (0 = illimite)", "type": "number", "required": false, "default": "0", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "default_bet", "label": "Mise par defaut", "type": "number", "required": false, "default": "10", "min": 1, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false, "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "chaos_enabled", "label": "Evenements chaos actifs", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "chaos_chance", "label": "Chance de chaos (%)", "type": "number", "required": false, "default": "18", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "chaos_enabled", "equals": "true"}},
        {"key": "daily_chaos_enabled", "label": "Chaos quotidien actif", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "chaos_enabled", "equals": "true"}},
        {"key": "daily_chaos_percent", "label": "Chaos quotidien : % vole", "type": "number", "required": false, "default": "20", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "daily_chaos_enabled", "equals": "true"}},

        {"key": "casino_enabled", "label": "Casino actif", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "casino_max_bet", "label": "Mise max casino", "type": "number", "required": false, "default": "500", "min": 1, "depends_on": {"key": "casino_enabled", "equals": "true"}},

        {"key": "steal_enabled", "label": "Vol actif", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "steal_success_rate", "label": "Taux reussite vol (%)", "type": "number", "required": false, "default": "30", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "steal_enabled", "equals": "true"}},
        {"key": "steal_cooldown_secs", "label": "Cooldown vol", "type": "number", "required": false, "default": "1800", "min": 0, "unit": "s", "depends_on": {"key": "steal_enabled", "equals": "true"}},

        {"key": "insurance_cost", "label": "Cout assurance", "type": "number", "required": false, "default": "50", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "insurance_duration_secs", "label": "Duree assurance", "type": "number", "required": false, "default": "3600", "min": 0, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "insurance_scam_rate", "label": "Taux arnaque assurance (%)", "type": "number", "required": false, "default": "5", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "cowardice_threshold", "label": "Seuil lachete (role poule)", "type": "number", "required": false, "default": "5", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "cowardice_penalty", "label": "Penalite lachete sur gains (%)", "type": "number", "required": false, "default": "20", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "refusal_penalty", "label": "Penalite refus (% de la mise)", "type": "number", "required": false, "default": "20", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "happy_hour_multiplier", "label": "Multiplicateur Happy Hour", "type": "number", "required": false, "default": "2", "min": 1, "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "shop_explosion_price", "label": "Prix Explosion", "type": "number", "required": false, "default": "200", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "shop_inversion_price", "label": "Prix Inversion", "type": "number", "required": false, "default": "500", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "shop_mindgame_price", "label": "Prix Mindgame", "type": "number", "required": false, "default": "150", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "shop_rage_price", "label": "Prix Rage", "type": "number", "required": false, "default": "100", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "shop_surprise_price", "label": "Prix Attaque Surprise", "type": "number", "required": false, "default": "300", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "shop_double_coup_price", "label": "Prix Double Coup", "type": "number", "required": false, "default": "250", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "shop_coup_traitre_price", "label": "Prix Coup Traitre", "type": "number", "required": false, "default": "350", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "combat_expiry_check_secs", "label": "Worker : intervalle scan combats expires", "type": "number", "required": false, "default": "5", "min": 1, "unit": "s", "description": "Tick du worker pour finaliser les combats /coude qui ont depasse leur fenetre de defense.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "combat_expiry_hours", "label": "Delai expiration combat", "type": "number", "required": false, "default": "24", "min": 1, "unit": "h", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "expiry_penalty_percent", "label": "Penalite expiration (% mise)", "type": "number", "required": false, "default": "20", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "refund_bets_on_expiry", "label": "Rembourser les paris a l''expiration", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "bet_delay_secs", "label": "Delai paris ouverts", "type": "number", "required": false, "default": "30", "min": 0, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'coude-bot';

-- 3) Supprime la definition du worker — disparait de la section
-- "Workers" dans la page Composants.
DELETE FROM bot_definitions WHERE bot_name = 'coude-worker';
