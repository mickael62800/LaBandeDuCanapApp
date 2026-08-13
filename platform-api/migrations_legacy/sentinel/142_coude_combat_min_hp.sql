-- Ajoute la cle combat_min_hp_pct au config_schema de coude-bot.
-- Empeche les combats avec des joueurs a 0 HP (ou trop bas) :
-- le jeu ne sait pas resoudre un combat ou quelqu un commence a 0 HP.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "combat_min_hp_pct", "label": "PV min (%) pour combattre", "type": "number", "required": false, "default": "40", "description": "Pourcentage minimum de PV requis pour que les deux combattants puissent engager un combat. 0 = desactive."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "combat_min_hp_pct"}]'::jsonb);
