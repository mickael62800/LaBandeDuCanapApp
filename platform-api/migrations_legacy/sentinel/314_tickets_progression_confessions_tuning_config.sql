-- ============================================================================
-- Exposition de reglages codes en dur (TICKETS / PROGRESSION / CONFESSIONS).
-- ============================================================================
-- Des bornes/valeurs jusqu'ici codees en dur deviennent reglables par serveur,
-- sur le modele des migrations 306/309/310 : ajout de cles au schema
-- (`bot_definitions.config_schema`) et, pour la fenetre de quota des
-- confessions, ajout d'une colonne a la table dediee `confession_config`.
--
-- Comportement : chaque valeur retombe sur son defaut historique -> AUCUN
-- changement tant que non reconfiguree. Des gardes cote code bornent les
-- valeurs (min <= max, plafond modal Discord 4000, multiplicateur >= 1.0,
-- fenetre >= 1h). Idempotent : cles ajoutees seulement si absentes du schema,
-- colonne ajoutee via IF NOT EXISTS.

-- TICKETS : bornes des champs de la modale de creation --------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "ticket_subject_min_len", "label": "Sujet — longueur min", "type": "number", "required": false, "default": "5", "min": 1, "max": 4000, "unit": "caracteres", "description": "Longueur minimale du champ Sujet de la modale de ticket. Doit rester <= au max (sinon les defauts 5/100 sont utilises)."},
    {"key": "ticket_subject_max_len", "label": "Sujet — longueur max", "type": "number", "required": false, "default": "100", "min": 1, "max": 4000, "unit": "caracteres", "description": "Longueur maximale du champ Sujet de la modale de ticket (plafonnee a 4000, limite Discord)."},
    {"key": "ticket_desc_min_len", "label": "Description — longueur min", "type": "number", "required": false, "default": "10", "min": 1, "max": 4000, "unit": "caracteres", "description": "Longueur minimale du champ Description de la modale de ticket. Doit rester <= au max (sinon les defauts 10/2000 sont utilises)."},
    {"key": "ticket_desc_max_len", "label": "Description — longueur max", "type": "number", "required": false, "default": "2000", "min": 1, "max": 4000, "unit": "caracteres", "description": "Longueur maximale du champ Description de la modale de ticket (plafonnee a 4000, limite Discord)."}
]'::jsonb
WHERE bot_name = 'ticket-bot'
  AND NOT (config_schema @> '[{"key": "ticket_subject_min_len"}]'::jsonb);

-- PROGRESSION : multiplicateur XP de streak -------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "streak_bonus_per_week", "label": "Streak — bonus XP par semaine", "type": "number", "required": false, "default": "0.1", "min": 0, "max": 10, "description": "Bonus de multiplicateur XP ajoute par semaine complete de streak (0.1 = +10% par 7 jours consecutifs)."},
    {"key": "streak_max_multiplier", "label": "Streak — multiplicateur max", "type": "number", "required": false, "default": "1.5", "min": 1, "max": 10, "description": "Plafond du multiplicateur XP de streak (1.5 = +50% max). Garde >= 1.0 (ne reduit jamais l XP)."}
]'::jsonb
WHERE bot_name = 'progression-bot'
  AND NOT (config_schema @> '[{"key": "streak_bonus_per_week"}]'::jsonb);

-- CONFESSIONS : archivage du thread + longueur de la raison de signalement ------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "thread_archive_minutes", "label": "Thread de reponses — archivage auto", "type": "enum", "required": false, "default": "60",
     "options": [
       {"value": "60", "label": "1 heure"},
       {"value": "1440", "label": "1 jour"},
       {"value": "4320", "label": "3 jours"},
       {"value": "10080", "label": "1 semaine"}
     ],
     "description": "Delai d inactivite apres lequel le thread de reponses d une confession s archive. Un thread archive se rouvre automatiquement a la prochaine reponse. Discord n autorise que ces 4 paliers."},
    {"key": "report_reason_max_len", "label": "Signalement — longueur max de la raison", "type": "number", "required": false, "default": "500", "min": 1, "max": 4000, "unit": "caracteres", "description": "Longueur maximale du champ Raison de la modale de signalement d une confession (plafonnee a 4000, limite Discord)."}
]'::jsonb
WHERE bot_name = 'confessions'
  AND NOT (config_schema @> '[{"key": "thread_archive_minutes"}]'::jsonb);

-- CONFESSIONS : fenetre glissante du quota `max_per_day` (table dediee) ----------
-- Le quota est applique cote domaine (ManageConfessionsService) a partir de la
-- config `confession_config`, comme `max_per_day`. On ajoute donc une colonne
-- (defaut 24h) plutot qu une cle de schema.
ALTER TABLE confession_config
    ADD COLUMN IF NOT EXISTS quota_window_hours INT NOT NULL DEFAULT 24;
