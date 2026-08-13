-- ============================================================================
-- Casino tuning — exposition des valeurs de reglage par serveur (3 casinos).
-- ============================================================================
-- Jusqu'ici, les payouts/poids de la Roue, le seuil de tirage du dealer au
-- blackjack, et les timings d'animation slot/roue etaient codes en dur dans le
-- domaine. On les rend reglables par serveur.
--
-- Comportement : le domaine reste PUR (config passee en entree). Chaque cle
-- retombe sur le defaut historique si absente/malformee -> AUCUN changement de
-- comportement tant que non reconfigure. Garde-fous appliques cote code (clamp
-- au parsing) ET via min/max ici quand pertinent :
--   - Roue : payout par case clampe a ±50000 ; somme des poids > 0 (sinon
--     restauration des poids par defaut).
--   - Blackjack : dealer_hit_threshold clampe a 16..=20.
--   - Slot : frames 1..=9, delai 250..=5000 ms.
--   - Roue : animation 500..=15000 ms.
--
-- Idempotent : chaque bloc n'ajoute ses cles que si absentes du schema.

-- ── A. wheel-bot : payout + poids de chacune des 10 cases (20 cles) ──────────
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "wheel_blanche_payout", "label": "Case Blanche — gain", "type": "number", "required": false, "default": "0", "unit": "coins", "min": -50000, "max": 50000, "description": "Gain (ou perte) de la case Blanche.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_blanche_weight", "label": "Case Blanche — poids", "type": "number", "required": false, "default": "25", "min": 0, "max": 1000, "description": "Poids RNG de la case Blanche (plus grand = sort plus souvent).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_pq_payout", "label": "Case PQ — gain", "type": "number", "required": false, "default": "50", "unit": "coins", "min": -50000, "max": 50000, "description": "Gain (ou perte) de la case PQ.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_pq_weight", "label": "Case PQ — poids", "type": "number", "required": false, "default": "20", "min": 0, "max": 1000, "description": "Poids RNG de la case PQ.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_sieste_payout", "label": "Case Sieste — gain", "type": "number", "required": false, "default": "200", "unit": "coins", "min": -50000, "max": 50000, "description": "Gain (ou perte) de la case Sieste.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_sieste_weight", "label": "Case Sieste — poids", "type": "number", "required": false, "default": "15", "min": 0, "max": 1000, "description": "Poids RNG de la case Sieste.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_colis_payout", "label": "Case Colis — gain", "type": "number", "required": false, "default": "500", "unit": "coins", "min": -50000, "max": 50000, "description": "Gain (ou perte) de la case Colis.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_colis_weight", "label": "Case Colis — poids", "type": "number", "required": false, "default": "12", "min": 0, "max": 1000, "description": "Poids RNG de la case Colis.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_trefle_payout", "label": "Case Trefle — gain", "type": "number", "required": false, "default": "1000", "unit": "coins", "min": -50000, "max": 50000, "description": "Gain (ou perte) de la case Trefle.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_trefle_weight", "label": "Case Trefle — poids", "type": "number", "required": false, "default": "10", "min": 0, "max": 1000, "description": "Poids RNG de la case Trefle.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_couronne_payout", "label": "Case Couronne — gain", "type": "number", "required": false, "default": "1500", "unit": "coins", "min": -50000, "max": 50000, "description": "Gain (ou perte) de la case Couronne.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_couronne_weight", "label": "Case Couronne — poids", "type": "number", "required": false, "default": "7", "min": 0, "max": 1000, "description": "Poids RNG de la case Couronne.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_ruine_payout", "label": "Case Ruine — gain", "type": "number", "required": false, "default": "-500", "unit": "coins", "min": -50000, "max": 50000, "description": "Gain (ou perte) de la case Ruine.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_ruine_weight", "label": "Case Ruine — poids", "type": "number", "required": false, "default": "5", "min": 0, "max": 1000, "description": "Poids RNG de la case Ruine.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_jackpot_payout", "label": "Case Jackpot — gain", "type": "number", "required": false, "default": "5000", "unit": "coins", "min": -50000, "max": 50000, "description": "Gain (ou perte) de la case Jackpot.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_jackpot_weight", "label": "Case Jackpot — poids", "type": "number", "required": false, "default": "3", "min": 0, "max": 1000, "description": "Poids RNG de la case Jackpot.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_bombe_payout", "label": "Case Bombe — gain", "type": "number", "required": false, "default": "-2000", "unit": "coins", "min": -50000, "max": 50000, "description": "Gain (ou perte) de la case Bombe.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_bombe_weight", "label": "Case Bombe — poids", "type": "number", "required": false, "default": "2", "min": 0, "max": 1000, "description": "Poids RNG de la case Bombe.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_licorne_payout", "label": "Case Licorne — gain", "type": "number", "required": false, "default": "10000", "unit": "coins", "min": -50000, "max": 50000, "description": "Gain (ou perte) de la case Licorne (jackpot rare).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_licorne_weight", "label": "Case Licorne — poids", "type": "number", "required": false, "default": "1", "min": 0, "max": 1000, "description": "Poids RNG de la case Licorne.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "wheel_spin_animation_ms", "label": "Duree animation du spin", "type": "number", "required": false, "default": "4000", "unit": "ms", "min": 500, "max": 15000, "description": "Duree du suspense avant l affichage du resultat de la Roue.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'wheel-bot'
  AND NOT (config_schema @> '[{"key": "wheel_blanche_payout"}]'::jsonb);

-- ── B. blackjack-bot : seuil de tirage du dealer ─────────────────────────────
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "dealer_hit_threshold", "label": "Seuil de tirage du dealer", "type": "number", "required": false, "default": "17", "min": 16, "max": 20, "description": "Le dealer tire tant que son score est inferieur a ce seuil (regle standard : 17). Borne a 16-20.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'blackjack-bot'
  AND NOT (config_schema @> '[{"key": "dealer_hit_threshold"}]'::jsonb);

-- ── C. slot-bot : timings d'animation du reveal ──────────────────────────────
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "spin_animation_total_frames", "label": "Frames de revele", "type": "number", "required": false, "default": "3", "min": 1, "max": 9, "description": "Nombre d etapes de revele progressif des rouleaux avant le resultat.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "spin_animation_frame_delay_ms", "label": "Delai entre frames", "type": "number", "required": false, "default": "2000", "unit": "ms", "min": 250, "max": 5000, "description": "Delai entre 2 etapes de revele de l animation slot.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'slot-bot'
  AND NOT (config_schema @> '[{"key": "spin_animation_total_frames"}]'::jsonb);
