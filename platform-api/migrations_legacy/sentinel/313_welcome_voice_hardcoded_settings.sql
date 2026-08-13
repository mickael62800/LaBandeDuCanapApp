-- ============================================================================
-- Welcome + Voice — exposition de reglages jusqu'ici codes en dur.
-- ============================================================================
-- Plusieurs valeurs de presentation/interaction etaient codees en dur cote bot :
--   welcome-bot : bornes de la verification d'age, multiplicateur du ban
--                 sous-age, couleurs des embeds depart et reglement.
--   voice-bot   : intervalle du sweep AFK, presets de duree de voice-ban,
--                 limite max de membres d'un salon.
-- On ajoute les cles manquantes aux config_schema respectifs. Le bot lit chaque
-- cle via get_guild_config_for("<bot>") avec le defaut = valeur historique et un
-- garde a la lecture -> AUCUN changement de comportement tant que non reconfigure.
--
-- Idempotent : chaque bloc n'ajoute ses cles que si absentes du schema.

-- Welcome ---------------------------------------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "age_min", "label": "Verification age — age minimum saisissable", "type": "number", "required": false, "default": "5", "min": 0, "max": 120, "description": "Borne basse acceptee dans le formulaire de verification d age (valeurs plus petites = rejet de la saisie).", "depends_on": {"key": "age_check_enabled", "equals": "true"}},
    {"key": "age_max", "label": "Verification age — age maximum saisissable", "type": "number", "required": false, "default": "120", "min": 0, "max": 200, "description": "Borne haute acceptee dans le formulaire de verification d age (valeurs plus grandes = rejet de la saisie).", "depends_on": {"key": "age_check_enabled", "equals": "true"}},
    {"key": "age_ban_days_per_year", "label": "Verification age — jours de ban par annee manquante", "type": "number", "required": false, "default": "365", "min": 1, "max": 366, "description": "Duree (en jours) du ban temporaire par annee manquante sous l age minimum. 365 = un an par annee.", "depends_on": {"key": "age_check_enabled", "equals": "true"}},
    {"key": "leave_embed_color", "label": "Couleur embed depart (hex)", "type": "text", "required": false, "default": "e74c3c", "description": "Code couleur hex sans # de l embed de message de depart (ex: e74c3c)."},
    {"key": "rules_embed_color", "label": "Couleur embed reglement (hex)", "type": "text", "required": false, "default": "5865f2", "description": "Code couleur hex sans # du panneau de reglement (ex: 5865f2)."}
]'::jsonb
WHERE bot_name = 'welcome-bot'
  AND NOT (config_schema @> '[{"key": "age_min"}]'::jsonb);

-- Voice -----------------------------------------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "afk_sweep_interval_secs", "label": "AFK — intervalle du balayage", "type": "number", "required": false, "default": "60", "min": 30, "max": 600, "unit": "s", "description": "Frequence a laquelle le bot verifie les membres AFK a deplacer (lecture globale : premiere guild configuree)."},
    {"key": "voice_ban_preset_secs", "label": "Voice-ban — presets de duree (CSV secondes)", "type": "text", "required": false, "default": "300,3600,86400", "description": "Trois durees (en secondes) des boutons de voice-ban, separees par des virgules. Defaut : 300,3600,86400 (5 min, 1 h, 24 h)."},
    {"key": "voice_max_user_limit", "label": "Salon vocal — limite max de membres", "type": "number", "required": false, "default": "99", "min": 1, "max": 99, "description": "Limite maximale de membres autorisee pour un salon vocal (plafond Discord : 99)."}
]'::jsonb
WHERE bot_name = 'voice-bot'
  AND NOT (config_schema @> '[{"key": "afk_sweep_interval_secs"}]'::jsonb);
