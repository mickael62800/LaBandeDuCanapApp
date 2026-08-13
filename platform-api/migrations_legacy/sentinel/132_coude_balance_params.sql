-- Phase 132 : parametres d equilibrage de Coup de Coude
--
-- Expose via bot_guild_config les valeurs magiques du gameplay
-- (combat, vol, braquage) pour qu elles soient editables depuis l UI
-- sans rebuild. Les getters cote bot sont ajoutes dans
-- `sentinel-bot/src/modules/coude/guild_config.rs`. La plupart
-- des regles sont consommees par l API (moteur de combat) ;
-- `steal_failure_penalty_pct` est deja applique cote bot dans
-- `commands/voler.rs`.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "surprise_min_hp_percent", "label": "PV min attaquant pour Surprise (%)", "type": "number", "required": false, "default": "40", "description": "Pourcentage de PV max minimum requis pour qu un attaquant puisse utiliser l item Surprise. 0 = desactive."},
  {"key": "surprise_allow_defender_counter", "label": "Defenseur peut contrer Surprise avec Explosion", "type": "boolean", "required": false, "default": "true", "description": "Si true, une cible qui possede Explosion peut l utiliser en reponse a une attaque Surprise."},
  {"key": "steal_max_active_boosts", "label": "Max boosts voleur actifs simultanes", "type": "number", "required": false, "default": "3", "description": "Nombre maximum d abonnements boost voleur simultanes par joueur. 0 = illimite."},
  {"key": "steal_failure_penalty_pct", "label": "Penalite coins sur vol rate (%)", "type": "number", "required": false, "default": "20", "description": "Pourcentage de coins que le voleur perd si son vol echoue."},
  {"key": "braquage_tools_consumed_success_pct", "label": "% outils consommes si braquage reussi", "type": "number", "required": false, "default": "50", "description": "Pourcentage d outils utilises consommes au hasard quand le braquage reussit."},
  {"key": "braquage_tools_consumed_fail_pct", "label": "% outils consommes si braquage rate", "type": "number", "required": false, "default": "25", "description": "Pourcentage d outils utilises consommes au hasard quand le braquage echoue."},
  {"key": "double_coup_mode", "label": "Mode agregation Double Coup", "type": "text", "required": false, "default": "median", "description": "Strategie d agregation des deux d20 de l item Double Coup : max, median ou min."},
  {"key": "rage_atk_bonus_pct", "label": "Bonus ATK Rage (%)", "type": "number", "required": false, "default": "40", "description": "Pourcentage de bonus d attaque applique par l item Rage."},
  {"key": "rage_def_malus_pct", "label": "Malus DEF Rage (%)", "type": "number", "required": false, "default": "15", "description": "Pourcentage de malus de defense applique par l item Rage."},
  {"key": "coup_traitre_def_malus_pct", "label": "Malus DEF Coup Traitre (%)", "type": "number", "required": false, "default": "40", "description": "Pourcentage de malus de defense applique au defenseur par l item Coup Traitre."},
  {"key": "bouclier_def_bonus_pct", "label": "Bonus DEF Bouclier (%)", "type": "number", "required": false, "default": "20", "description": "Pourcentage de bonus de defense applique par l item Bouclier."},
  {"key": "poison_damage_per_round", "label": "Degats Poison par round (PV)", "type": "number", "required": false, "default": "5", "description": "PV perdus par round par un joueur empoisonne."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "surprise_min_hp_percent"}]'::jsonb);
