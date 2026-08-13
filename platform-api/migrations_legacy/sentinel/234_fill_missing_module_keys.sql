-- Audit code vs schema : restore les cles consommees mais absentes du
-- schema UI (regression de mes refontes precedentes + lacunes anciennes).
--
-- Modules concernes :
--   - voice-bot : panel_post_enabled, voice_anchor_category_id (drop accidentel mig 225)
--   - coude-bot : 41 cles de gameplay (assurance tiers, casino limits,
--     gift, prank, prestige, rage, repos, steal limits, surprise,
--     ultimate, voler) jamais exposees malgre lecture par le code.
--   - moderation-bot : ban_delete_message_days, max_mute_duration_secs
--     (absentes apres mig 223 cleanup).
--   - security-bot : verifier min_account_age_secs (deja en schema 027).

-- ── voice-bot : restore 2 cles drop par mig 225 ──────────────────
UPDATE bot_definitions SET
    config_schema = config_schema || '[
        {"key": "panel_post_enabled", "label": "Panneau de controle dans le chat vocal", "type": "boolean", "required": false, "default": "true", "description": "Si OFF, aucun panneau de controle n est poste a la creation d un salon vocal temporaire.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "voice_anchor_category_id", "label": "Categorie ancre vocaux", "type": "channel", "required": false, "description": "Categorie Discord ou les salons vocaux temporaires sont crees (positionnement). Si vide, places sous les lobby creators.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'voice-bot'
  AND NOT (config_schema @> '[{"key": "panel_post_enabled"}]'::jsonb);

-- ── moderation-bot : restore 2 cles ──────────────────────────────
UPDATE bot_definitions SET
    config_schema = config_schema || '[
        {"key": "ban_delete_message_days", "label": "Nb jours messages supprimes au ban", "type": "number", "required": false, "default": "0", "min": 0, "max": 7, "unit": "j", "description": "Lors d un ban, supprime les messages des N derniers jours. 0 = aucun, 7 = max Discord.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "max_mute_duration_secs", "label": "Duree max d un mute", "type": "number", "required": false, "default": "2419200", "min": 60, "max": 2419200, "unit": "s", "description": "Plafond de duree d un mute. Max Discord = 28 jours (2419200s).", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'moderation-bot'
  AND NOT (config_schema @> '[{"key": "ban_delete_message_days"}]'::jsonb);

-- ── coude-bot : ajout 41 cles gameplay ───────────────────────────
UPDATE bot_definitions SET
    config_schema = config_schema || '[
        {"key": "combat_expire_secs", "label": "Expiration combat (secondes)", "type": "number", "required": false, "default": "86400", "min": 60, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "assurance_tier_day_secs", "label": "Assurance jour : duree", "type": "number", "required": false, "default": "86400", "min": 60, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "assurance_tier_day_mult", "label": "Assurance jour : multiplicateur cout", "type": "number", "required": false, "default": "1.0", "min": 0.1, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "assurance_tier_week_secs", "label": "Assurance semaine : duree", "type": "number", "required": false, "default": "604800", "min": 60, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "assurance_tier_week_mult", "label": "Assurance semaine : multiplicateur cout", "type": "number", "required": false, "default": "5.0", "min": 0.1, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "assurance_tier_month_secs", "label": "Assurance mois : duree", "type": "number", "required": false, "default": "2592000", "min": 60, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "assurance_tier_month_mult", "label": "Assurance mois : multiplicateur cout", "type": "number", "required": false, "default": "15.0", "min": 0.1, "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "casino_cooldown_secs", "label": "Casino : cooldown entre parties", "type": "number", "required": false, "default": "30", "min": 0, "unit": "s", "depends_on": {"key": "casino_enabled", "equals": "true"}},
        {"key": "casino_max_daily", "label": "Casino : parties max/jour", "type": "number", "required": false, "default": "0", "min": 0, "description": "0 = illimite.", "depends_on": {"key": "casino_enabled", "equals": "true"}},
        {"key": "casino_max_daily_gain", "label": "Casino : gain max/jour", "type": "number", "required": false, "default": "0", "min": 0, "unit": "coins", "description": "0 = illimite.", "depends_on": {"key": "casino_enabled", "equals": "true"}},

        {"key": "steal_max_daily", "label": "Vol : tentatives max/jour", "type": "number", "required": false, "default": "0", "min": 0, "description": "0 = illimite.", "depends_on": {"key": "steal_enabled", "equals": "true"}},
        {"key": "steal_max_active_boosts", "label": "Vol : nb boosts actifs simultanes", "type": "number", "required": false, "default": "3", "min": 0, "max": 10, "depends_on": {"key": "steal_enabled", "equals": "true"}},
        {"key": "steal_failure_penalty_pct", "label": "Vol : penalite echec (%)", "type": "number", "required": false, "default": "10", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "steal_enabled", "equals": "true"}},
        {"key": "voler_min_target_coins", "label": "Vol : coins min de la cible", "type": "number", "required": false, "default": "100", "min": 0, "unit": "coins", "depends_on": {"key": "steal_enabled", "equals": "true"}},

        {"key": "gift_cooldown_secs", "label": "Don : cooldown", "type": "number", "required": false, "default": "3600", "min": 0, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "gift_min_coins", "label": "Don : montant minimum", "type": "number", "required": false, "default": "10", "min": 1, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "gift_min_coins_after", "label": "Don : coins min restants donneur", "type": "number", "required": false, "default": "50", "min": 0, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "gift_tax_percent", "label": "Don : taxe (%)", "type": "number", "required": false, "default": "10", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "prank_appel_cost", "label": "Prank /appel : cout", "type": "number", "required": false, "default": "100", "min": 0, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "prank_braquage_cost", "label": "Prank /braquage : cout", "type": "number", "required": false, "default": "150", "min": 0, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "prank_scoop_cost", "label": "Prank /scoop : cout", "type": "number", "required": false, "default": "50", "min": 0, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "braquage_tools_consumed_success_pct", "label": "Braquage : outils consommes succes (%)", "type": "number", "required": false, "default": "30", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "braquage_tools_consumed_fail_pct", "label": "Braquage : outils consommes echec (%)", "type": "number", "required": false, "default": "60", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "rage_atk_bonus_pct", "label": "Rage : bonus attaque (%)", "type": "number", "required": false, "default": "30", "min": 0, "max": 200, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "rage_def_malus_pct", "label": "Rage : malus defense (%)", "type": "number", "required": false, "default": "20", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "bouclier_def_bonus_pct", "label": "Bouclier : bonus defense (%)", "type": "number", "required": false, "default": "40", "min": 0, "max": 200, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "coup_traitre_def_malus_pct", "label": "Coup traitre : malus defense (%)", "type": "number", "required": false, "default": "30", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "double_coup_mode", "label": "Double coup : mode", "type": "text", "required": false, "default": "additive", "description": "Mode de calcul du double coup (additive / multiplicative).", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "surprise_allow_defender_counter", "label": "Attaque surprise : defenseur peut riposter", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "surprise_min_hp_percent", "label": "Attaque surprise : HP min cible (%)", "type": "number", "required": false, "default": "20", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "poison_damage_per_round", "label": "Poison : degats par round", "type": "number", "required": false, "default": "5", "min": 0, "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "tout_ou_rien_animation_secs", "label": "Tout-ou-rien : duree animation", "type": "number", "required": false, "default": "10", "min": 0, "unit": "s", "depends_on": {"key": "casino_enabled", "equals": "true"}},

        {"key": "repos_cooldown_hours", "label": "Repos : cooldown", "type": "number", "required": false, "default": "24", "min": 1, "unit": "h", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "class_change_cost", "label": "Cout changement de classe", "type": "number", "required": false, "default": "1000", "min": 0, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "reset_stats_cost", "label": "Cout reset des stats", "type": "number", "required": false, "default": "500", "min": 0, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "prestige_unlock_level", "label": "Niveau pour debloquer le prestige", "type": "number", "required": false, "default": "50", "min": 1, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "ultimate_unlock_level", "label": "Niveau pour debloquer l ultimate", "type": "number", "required": false, "default": "30", "min": 1, "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "coalition_cost_per_member", "label": "Coalition : cout par membre", "type": "number", "required": false, "default": "50", "min": 0, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "contribute_prime_min", "label": "Contribution prime : minimum", "type": "number", "required": false, "default": "10", "min": 0, "unit": "coins", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "cancel_penalty", "label": "Penalite annulation combat (%)", "type": "number", "required": false, "default": "10", "min": 0, "max": 100, "unit": "%", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "mise_pick_suggested_percent", "label": "Mise suggeree (%)", "type": "number", "required": false, "default": "10", "min": 0, "max": 100, "unit": "%", "description": "Pourcentage des coins suggere comme mise par defaut dans le UI /coude.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "combat_expire_secs"}]'::jsonb);
