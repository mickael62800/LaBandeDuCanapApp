-- ============================================================================
-- Coup de Coude — exposition des reglages ECONOMY (tuning per-guild).
-- ============================================================================
-- Les valeurs de balance ECONOMY (XP de combat, % de vol, tout-ou-rien,
-- braquage, cout des maledictions, frais, prize pool de tournoi) etaient codees
-- en dur dans le domaine (`resolution_rules`, `steal::roll`, `tout_ou_rien`,
-- `heist`, `curse`, `tournament`). On les rend reglables par serveur via la
-- config `coude-bot`, sur le modele de la migration 309 (automod scoring).
--
-- Comportement : le domaine reste PUR (la config est passee en entree via
-- `CoudeEconomyConfig`). Chaque cle retombe sur le defaut historique si
-- absente/malformee -> AUCUN changement de comportement tant que non
-- reconfiguree. Des gardes cote application bornent les valeurs (pourcentages
-- 0..100, probabilites 0..1, multiplicateurs planches, montants >= 0, min<=max).
--
-- Valeurs naturelles (ex. "15", "0.5"), decimales tolerees.
-- Idempotent : les cles ne sont ajoutees que si absentes du schema.

-- Combat XP --------------------------------------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "combat_xp_winner_base", "label": "XP — vainqueur (base)", "type": "number", "required": false, "default": "15", "min": 0, "max": 100000, "unit": "XP", "description": "XP attribue au vainqueur d un combat (sans bonus Giant Killer)."},
    {"key": "combat_xp_winner_underdog", "label": "XP — vainqueur underdog (Giant Killer)", "type": "number", "required": false, "default": "30", "min": 0, "max": 100000, "unit": "XP", "description": "XP attribue au vainqueur lorsqu il a battu un adversaire de >= 3 niveaux au-dessus (bonus Giant Killer)."},
    {"key": "combat_xp_loser", "label": "XP — perdant (consolation)", "type": "number", "required": false, "default": "5", "min": 0, "max": 100000, "unit": "XP", "description": "XP de consolation attribue au perdant d un combat."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "combat_xp_winner_base"}]'::jsonb);

-- Vol (/voler) : % de wallet vole selon statut AFK/actif de la cible ----------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "steal_afk_min_pct", "label": "Vol — % min (cible AFK)", "type": "number", "required": false, "default": "10", "min": 0, "max": 100, "unit": "%", "description": "Borne basse du pourcentage du portefeuille vole quand la cible est AFK (n a pas defendu). Doit rester <= au max AFK."},
    {"key": "steal_afk_max_pct", "label": "Vol — % max (cible AFK)", "type": "number", "required": false, "default": "15", "min": 0, "max": 100, "unit": "%", "description": "Borne haute du pourcentage du portefeuille vole quand la cible est AFK."},
    {"key": "steal_active_min_pct", "label": "Vol — % min (cible active)", "type": "number", "required": false, "default": "15", "min": 0, "max": 100, "unit": "%", "description": "Borne basse du pourcentage du portefeuille vole quand la cible a defendu. Doit rester <= au max actif."},
    {"key": "steal_active_max_pct", "label": "Vol — % max (cible active)", "type": "number", "required": false, "default": "25", "min": 0, "max": 100, "unit": "%", "description": "Borne haute du pourcentage du portefeuille vole quand la cible a defendu."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "steal_afk_min_pct"}]'::jsonb);

-- Tout-ou-rien -----------------------------------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "tout_ou_rien_win_probability", "label": "Tout-ou-rien — probabilite de gain", "type": "number", "required": false, "default": "0.5", "min": 0, "max": 1, "description": "Probabilite de gagner le tout-ou-rien (0.5 = 50/50). Bornee a [0, 1]."},
    {"key": "tout_ou_rien_win_multiplier", "label": "Tout-ou-rien — multiplicateur de gain", "type": "number", "required": false, "default": "2", "min": 1, "max": 1000, "description": "Multiplicateur applique au wallet en cas de victoire (2 = solde double). Plancher a 1 (jamais < 1)."},
    {"key": "tout_ou_rien_loss_keep_pct", "label": "Tout-ou-rien — fraction conservee en cas de defaite", "type": "number", "required": false, "default": "0.2", "min": 0, "max": 1, "description": "Fraction du wallet conservee en cas de defaite (0.2 = le joueur garde 20%). Bornee a [0, 1]."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "tout_ou_rien_win_probability"}]'::jsonb);

-- Braquage (/braquage) --------------------------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "heist_base_success_pct", "label": "Braquage — chance de base", "type": "number", "required": false, "default": "5", "min": 0, "max": 100, "unit": "%", "description": "Taux de reussite de base d un braquage sans aucun outil. Doit rester <= au plafond."},
    {"key": "heist_max_success_pct", "label": "Braquage — plafond de reussite", "type": "number", "required": false, "default": "55", "min": 0, "max": 100, "unit": "%", "description": "Plafond du taux de reussite d un braquage (base + bonus outils)."},
    {"key": "heist_gain_min_pct", "label": "Braquage — % min du butin", "type": "number", "required": false, "default": "30", "min": 0, "max": 100, "unit": "%", "description": "Borne basse du pourcentage de la caisse vole en cas de succes. Doit rester <= au max."},
    {"key": "heist_gain_max_pct", "label": "Braquage — % max du butin", "type": "number", "required": false, "default": "75", "min": 0, "max": 100, "unit": "%", "description": "Borne haute du pourcentage de la caisse vole en cas de succes."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "heist_base_success_pct"}]'::jsonb);

-- Maledictions / frais ---------------------------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "curse_cost_coins", "label": "Malediction — cout de lancement", "type": "number", "required": false, "default": "300", "min": 0, "max": 1000000000, "unit": "coins", "description": "Cout en coins d une malediction classique (/maudire). Les sabotages nommes gardent leur cout dedie."},
    {"key": "curse_lift_multiplier", "label": "Malediction — multiplicateur de levee", "type": "number", "required": false, "default": "2", "min": 1, "max": 1000, "description": "Multiplicateur applique au cout de lancement pour lever une malediction (2 = double). Plancher a 1."},
    {"key": "leaky_wallet_fee_coins", "label": "Portefeuille troue — frais par transaction", "type": "number", "required": false, "default": "10", "min": 0, "max": 1000000000, "unit": "coins", "description": "Frais fixes preleves sur chaque don sous l effet Portefeuille troue."},
    {"key": "fausse_assurance_fee_coins", "label": "Fausse assurance — frais additionnels", "type": "number", "required": false, "default": "200", "min": 0, "max": 1000000000, "unit": "coins", "description": "Frais additionnels preleves a la cible et rediriges au saboteur quand Fausse assurance se declenche."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "curse_cost_coins"}]'::jsonb);

-- Tournoi ----------------------------------------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "tournament_prize_pool_pct", "label": "Tournoi — % de la caisse pour le prize pool", "type": "number", "required": false, "default": "10", "min": 0, "max": 100, "unit": "%", "description": "Pourcentage du solde de la caisse communautaire constituant le prize pool estime du tournoi hebdomadaire."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "tournament_prize_pool_pct"}]'::jsonb);
