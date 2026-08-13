-- Tamagotchi — config_schema complet (page Composants). Tout parametrable.

UPDATE bot_definitions
SET config_schema = '[
    {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active le jeu Tamagotchi."},

    {"key": "tama_category_id", "label": "Categorie des salons prives", "type": "category", "required": false, "description": "Categorie ou sont crees les salons prives des compagnons (un par joueur). Vide = racine du serveur.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "panel_channel_id", "label": "Salon du panneau public", "type": "channel", "required": false, "description": "Salon ou est poste le panneau Ouvrir mon compagnon (via /tama-setup).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "log_channel_id", "label": "Salon des logs", "type": "channel", "required": false, "description": "Salon textuel ou sont logges les evenements importants (naissances, morts, combats). Vide = pas de logs.", "depends_on": {"key": "enabled", "equals": "true"}},

    {"key": "hunger_decay_per_hour", "label": "Baisse de la faim / h", "type": "number", "required": false, "default": "8", "min": 0, "max": 100, "description": "Points de FAIM perdus par heure.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "happiness_decay_per_hour", "label": "Baisse du bonheur / h", "type": "number", "required": false, "default": "5", "min": 0, "max": 100, "description": "Points de BONHEUR perdus par heure.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "energy_decay_per_hour", "label": "Baisse de l energie / h", "type": "number", "required": false, "default": "6", "min": 0, "max": 100, "description": "Points d ENERGIE perdus par heure (hors actions).", "depends_on": {"key": "enabled", "equals": "true"}},

    {"key": "low_gauge_malus_threshold", "label": "Seuil malus (jauge basse)", "type": "number", "required": false, "default": "20", "min": 0, "max": 100, "description": "Sous ce niveau de jauge, malus de stats en combat.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "sick_after_hours", "label": "Maladie apres (faim a 0)", "type": "number", "required": false, "default": "12", "min": 1, "max": 240, "unit": "heures", "description": "Si la faim reste a 0 pendant ce nombre d heures, le compagnon tombe malade.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "death_after_sick_hours", "label": "Mort apres maladie", "type": "number", "required": false, "default": "24", "min": 1, "max": 720, "unit": "heures", "description": "Si le compagnon reste malade (non soigne) ce nombre d heures, il meurt.", "depends_on": {"key": "enabled", "equals": "true"}},

    {"key": "xp_per_action", "label": "XP par action", "type": "number", "required": false, "default": "5", "min": 0, "max": 1000, "description": "XP gagnes a chaque action de soin.", "depends_on": {"key": "enabled", "equals": "true"}},

    {"key": "feed_cost", "label": "Cout Nourrir", "type": "number", "required": false, "default": "20", "min": 0, "max": 1000000, "unit": "coins", "description": "Cout en coins de l action Nourrir.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "feed_hunger_gain", "label": "Gain Faim (Nourrir)", "type": "number", "required": false, "default": "40", "min": 1, "max": 100, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "feed_cooldown_secs", "label": "Cooldown Nourrir", "type": "number", "required": false, "default": "1800", "min": 0, "max": 86400, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},

    {"key": "play_happiness_gain", "label": "Gain Bonheur (Jouer)", "type": "number", "required": false, "default": "30", "min": 1, "max": 100, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "play_energy_cost", "label": "Cout Energie (Jouer)", "type": "number", "required": false, "default": "10", "min": 0, "max": 100, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "play_cooldown_secs", "label": "Cooldown Jouer", "type": "number", "required": false, "default": "1800", "min": 0, "max": 86400, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},

    {"key": "sleep_energy_gain", "label": "Gain Energie (Dormir)", "type": "number", "required": false, "default": "60", "min": 1, "max": 100, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "sleep_cooldown_secs", "label": "Cooldown Dormir", "type": "number", "required": false, "default": "1020", "min": 0, "max": 86400, "unit": "s", "description": "Delai avant de pouvoir redormir (ex: 1020 = 17 min).", "depends_on": {"key": "enabled", "equals": "true"}},

    {"key": "cuddle_happiness_gain", "label": "Gain Bonheur (Caliner)", "type": "number", "required": false, "default": "15", "min": 1, "max": 100, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "cuddle_cooldown_secs", "label": "Cooldown Caliner", "type": "number", "required": false, "default": "3600", "min": 0, "max": 86400, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},

    {"key": "train_energy_cost", "label": "Cout Energie (Entrainer)", "type": "number", "required": false, "default": "25", "min": 0, "max": 100, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "train_cooldown_secs", "label": "Cooldown Entrainer", "type": "number", "required": false, "default": "7200", "min": 0, "max": 86400, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},

    {"key": "combat_energy_cost", "label": "Energie min pour Combat", "type": "number", "required": false, "default": "20", "min": 0, "max": 100, "description": "Energie minimale requise pour combattre (sinon epuise).", "depends_on": {"key": "enabled", "equals": "true"}},

    {"key": "visit_cooldown_secs", "label": "Cooldown Visiter", "type": "number", "required": false, "default": "6600", "min": 0, "max": 86400, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "visit_xp_reward", "label": "XP gagnes (visite recue)", "type": "number", "required": false, "default": "5", "min": 0, "max": 1000, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "visit_coins_reward", "label": "Coins gagnes (visite recue)", "type": "number", "required": false, "default": "5", "min": 0, "max": 100000, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "visit_max_per_day", "label": "Visites max / jour", "type": "number", "required": false, "default": "10", "min": 0, "max": 1000, "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'tamagotchi-bot';
