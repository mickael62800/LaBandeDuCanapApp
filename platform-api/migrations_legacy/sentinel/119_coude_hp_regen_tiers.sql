-- Coup de Coude : regen HP par paliers + seuil minimum pour combattre
--
-- La colonne hp_regen_per_hour existe depuis la migration 097 mais n'etait
-- jamais lue. Cette migration introduit 4 taux par palier de % HP :
-- - 0-25 % HP  -> 100 HP/h (sortie de KO rapide)
-- - 25-50 %    -> 50  HP/h
-- - 50-75 %    -> 30  HP/h
-- - 75-100 %   -> 10  HP/h
--
-- Applique aussi le seuil minimum de 10 % pour pouvoir lancer un /coude.

UPDATE bot_definitions
SET config_schema = config_schema::jsonb || '[
    {"key": "hp_regen_rate_0_25", "label": "HP/h palier 0-25%", "type": "number", "required": false, "default": "100"},
    {"key": "hp_regen_rate_25_50", "label": "HP/h palier 25-50%", "type": "number", "required": false, "default": "50"},
    {"key": "hp_regen_rate_50_75", "label": "HP/h palier 50-75%", "type": "number", "required": false, "default": "30"},
    {"key": "hp_regen_rate_75_100", "label": "HP/h palier 75-100%", "type": "number", "required": false, "default": "10"},
    {"key": "hp_regen_tick_secs", "label": "Frequence du job regen (secondes)", "type": "number", "required": false, "default": "300"}
]'::jsonb
WHERE bot_name = 'coude-bot';

-- Ramene le seuil minimum pour combattre a 10 % (anciennement 20 %).
-- On remplace l'entree existante en reconstruisant le tableau.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem->>'key' = 'hp_min_combat_pct'
             THEN jsonb_set(elem, '{default}', '"10"'::jsonb)
             ELSE elem
        END
    )
    FROM jsonb_array_elements(config_schema::jsonb) elem
)::jsonb
WHERE bot_name = 'coude-bot';
