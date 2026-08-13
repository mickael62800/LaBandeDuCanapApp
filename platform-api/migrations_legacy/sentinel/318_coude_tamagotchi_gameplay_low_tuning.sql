-- ============================================================================
-- Coup de Coude + Tamagotchi — exposition des reglages gameplay LOW.
-- ============================================================================
-- Derniers "magic numbers" gameplay non-monetaires codes en dur : cap
-- journalier du daily chaos, solde min eligible, probabilite des lignes de
-- flavor de combat, seuil de dette d honneur, ecart de niveaux underdog
-- (Giant Killer) cote coude-bot ; seuils visuels du sprite (fatigue /
-- mecontentement) cote tamagotchi-bot.
--
-- Comportement : le domaine reste PUR (les valeurs coude sont passees en
-- entree via `CoudeEconomyConfig`, les seuils tamagotchi sont lus cote bot).
-- Chaque cle retombe sur le defaut historique si absente/malformee -> AUCUN
-- changement de comportement tant que non reconfiguree. Des gardes bornent
-- les valeurs (probabilite 0..1, compteurs/coins/seuils >= 0, seuils
-- tamagotchi 0..100).
--
-- Idempotent : les cles ne sont ajoutees que si absentes du schema.

-- Coup de Coude — gameplay LOW ------------------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "daily_chaos_max_events", "label": "Daily chaos — cap journalier", "type": "number", "required": false, "default": "5", "min": 0, "max": 100, "unit": "evenements", "description": "Nombre maximum d evenements daily chaos declenches par jour et par serveur (0 = desactive)."},
    {"key": "min_coins_eligible", "label": "Daily chaos — solde minimum eligible", "type": "number", "required": false, "default": "10", "min": 0, "max": 1000000000, "unit": "coins", "description": "Solde minimum d un joueur pour etre tire au sort comme cible/gagnant du daily chaos."},
    {"key": "flavor_line_probability", "label": "Combat — probabilite ligne d ambiance", "type": "number", "required": false, "default": "0.2", "min": 0, "max": 1, "description": "Probabilite (0..1) qu une phrase d ambiance debile soit inseree a la fin d un round de combat. Aucune incidence mecanique."},
    {"key": "honor_debt_threshold", "label": "Dette d honneur — seuil de refus", "type": "number", "required": false, "default": "3", "min": 0, "max": 1000, "unit": "refus", "description": "Nombre de refus d un meme joueur au-dela duquel la dette d honneur peut etre invoquee."},
    {"key": "underdog_level_gap", "label": "Combat — ecart de niveaux Giant Killer", "type": "number", "required": false, "default": "3", "min": 0, "max": 1000, "unit": "niveaux", "description": "Ecart de niveaux minimum entre gagnant et perdant pour activer le bonus XP underdog (Giant Killer)."},
    {"key": "afk_defender_malus", "label": "Vol — malus defenseur AFK", "type": "number", "required": false, "default": "8", "min": 0, "max": 100, "description": "Malus applique au jet du defenseur lorsqu il ne se defend pas (AFK) face a une tentative de vol."},
    {"key": "animated_combat_mise_threshold", "label": "Combat — seuil d animation", "type": "number", "required": false, "default": "500", "min": 0, "max": 1000000000, "unit": "coins", "description": "Mise (coins) au-dessus de laquelle un combat est anime round par round. En dessous, le resultat est poste directement."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "daily_chaos_max_events"}]'::jsonb);

-- Tamagotchi — seuils visuels du sprite ---------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "sprite_tired_energy_threshold", "label": "Sprite — seuil energie fatigue", "type": "number", "required": false, "default": "25", "min": 0, "max": 100, "description": "Energie (<=) sous laquelle le sprite affiche l etat fatigue (dodo).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "sprite_unhappy_stat_threshold", "label": "Sprite — seuil faim/bonheur mecontent", "type": "number", "required": false, "default": "25", "min": 0, "max": 100, "description": "Faim ou bonheur (<=) sous lequel le sprite affiche l etat affame/mecontent.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'tamagotchi-bot'
  AND NOT (config_schema @> '[{"key": "sprite_tired_energy_threshold"}]'::jsonb);
