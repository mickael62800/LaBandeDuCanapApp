-- Cf. COUPE_AMELIORATIONS — expose les parametres des features 1.2 / 3.2 /
-- 3.3 / 4.1 / 4.4 / 4.5 dans le config_schema de coude-bot, pour que les
-- admins puissent les ajuster depuis la web UI (/components/config).
--
-- Aucun comportement par defaut ne change : tous les `default` ci-dessous
-- correspondent aux constantes Rust en place avant cette migration.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "mise_pick_suggested_percent", "label": "Mise par defaut suggeree (% du wallet)", "type": "number", "required": false, "default": "20", "description": "Cf. 1.2. Pourcentage du wallet propose comme mise rapide quand /coude est lance sans mise (clampe [min_bet, max_bet])."},

  {"key": "lucky_shield_enabled", "label": "Bouclier malchance du jour active", "type": "checkbox", "required": false, "default": "true", "description": "Cf. 4.1. Active la reduction de la 1ere defaite quotidienne. Desactive : 1ere defaite traitee comme une normale."},
  {"key": "lucky_shield_loss_percent", "label": "Bouclier malchance — perte conservee (%)", "type": "number", "required": false, "default": "50", "description": "Cf. 4.1. Sous bouclier, la perte est multipliee par ce pourcentage (50 = perte / 2). 100 = bouclier sans effet."},

  {"key": "assurance_extra_slot_level", "label": "Niveau debloquant 2e slot d''assurance", "type": "number", "required": false, "default": "5", "description": "Cf. 3.2. A partir de ce niveau, /assurance autorise 2 assurances actives concurrentes au lieu de 1. Mettre un niveau tres haut pour neutraliser le palier."},

  {"key": "prestige_unlock_level", "label": "Niveau requis pour /prestige", "type": "number", "required": false, "default": "25", "description": "Cf. 3.3. Niveau minimum pour pouvoir prestiger."},
  {"key": "prestige_max_count", "label": "Nombre max de prestiges", "type": "number", "required": false, "default": "5", "description": "Cf. 3.3. Plafond du compteur de prestige (= aussi cap d etoiles affichees)."},
  {"key": "prestige_gain_bonus_percent", "label": "Bonus de gain par prestige (%)", "type": "number", "required": false, "default": "5", "description": "Cf. 3.3. Multiplicateur additif sur les coins gagnes en combat : +N% par prestige (cap au nombre max de prestiges)."},

  {"key": "friendly_winner_xp", "label": "XP gagnant duel amical", "type": "number", "required": false, "default": "20", "description": "Cf. 4.5. XP attribue au gagnant d un duel /coude-amical (0 coin transfere)."},
  {"key": "friendly_loser_xp", "label": "XP perdant duel amical", "type": "number", "required": false, "default": "5", "description": "Cf. 4.5. XP attribue au perdant ou aux deux en cas d egalite."},

  {"key": "safety_net_trigger_coins", "label": "Filet — seuil d activation (coins)", "type": "number", "required": false, "default": "50", "description": "Cf. 4.4. Sous ce solde, le filet de securite s active automatiquement pour la duree configuree."},
  {"key": "safety_net_duration_hours", "label": "Filet — duree d activation (heures)", "type": "number", "required": false, "default": "72", "description": "Cf. 4.4. Combien d heures le filet reste actif apres declenchement."},
  {"key": "safety_net_loss_percent", "label": "Filet — perte conservee (%)", "type": "number", "required": false, "default": "50", "description": "Cf. 4.4. Sous filet, les pertes sont multipliees par ce pourcentage."},
  {"key": "safety_net_bet_gain_percent", "label": "Filet — boost gains paris (%)", "type": "number", "required": false, "default": "150", "description": "Cf. 4.4. Sous filet, les paris gagnants sont multiplies par ce pourcentage (150 = +50%)."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "mise_pick_suggested_percent"}]'::jsonb);
