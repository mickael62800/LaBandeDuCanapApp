-- Ajoute le champ "frequence de refresh de la carte" au panel web tamagotchi.
-- Pilote la tache de rafraichissement automatique des cartes dans Discord
-- (par serveur). Append non destructif au config_schema existant.
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "card_refresh_interval_minutes", "label": "Refresh carte (minutes)", "type": "number", "required": false, "default": "60", "min": 1, "max": 1440, "unit": "min", "description": "Frequence de rafraichissement automatique de la carte du compagnon dans Discord (re-edition du message).", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'tamagotchi-bot';
