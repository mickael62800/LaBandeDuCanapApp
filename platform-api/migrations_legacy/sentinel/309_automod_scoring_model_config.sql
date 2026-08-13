-- ============================================================================
-- Automod scoring model — exposition du modele de scoring (poids + seuils).
-- ============================================================================
-- Le scoring auto (poids par flag + seuils d'action) etait code en dur dans le
-- domaine (`scoring_service.rs`) avec des copies inline dupliquees dans les
-- chemins texte et image. On rend ce modele reglable par serveur via 14 cles
-- de la config `automod-bot` (la ou l'auto-scoring pilote les auto-actions).
--
-- Comportement : le domaine reste PUR (la config est passee en entree). Chaque
-- cle retombe sur le defaut historique si absente/malformee -> AUCUN changement
-- de comportement tant que non reconfigure. Une regle DB per-flag (`rules`)
-- reste prioritaire ; ces cles ne remplacent que le baseline hardcode.
--
-- Valeurs naturelles (ex. "7", pas x10), decimales tolerees, min 0.
-- Idempotent : chaque cle n'est ajoutee que si absente du schema.

-- Poids par flag ---------------------------------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "score_weight_spam", "label": "Scoring — poids spam", "type": "number", "required": false, "default": "3", "min": 0, "description": "Poids ajoute au score quand un message est detecte comme spam.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_weight_insult", "label": "Scoring — poids insulte", "type": "number", "required": false, "default": "5", "min": 0, "description": "Poids ajoute au score quand un message contient une insulte.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_weight_link", "label": "Scoring — poids lien", "type": "number", "required": false, "default": "1", "min": 0, "description": "Poids ajoute au score quand un message contient un lien.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_weight_phishing", "label": "Scoring — poids phishing", "type": "number", "required": false, "default": "7", "min": 0, "description": "Poids ajoute au score quand un message est detecte comme phishing.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_weight_nsfw", "label": "Scoring — poids NSFW (image)", "type": "number", "required": false, "default": "8", "min": 0, "description": "Poids ajoute au score quand une image est classee NSFW par la vision IA.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_weight_illicit", "label": "Scoring — poids illicite (image)", "type": "number", "required": false, "default": "9", "min": 0, "description": "Poids ajoute au score quand une image est classee illicite par la vision IA.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_weight_anger", "label": "Scoring — poids colere (IA texte)", "type": "number", "required": false, "default": "3", "min": 0, "description": "Poids de base du sentiment colere detecte par l IA texte (pondere par la confiance).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_weight_rage", "label": "Scoring — poids rage (IA texte)", "type": "number", "required": false, "default": "6", "min": 0, "description": "Poids de base du sentiment rage detecte par l IA texte (pondere par la confiance).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_weight_threat", "label": "Scoring — poids menace (IA texte)", "type": "number", "required": false, "default": "8", "min": 0, "description": "Poids de base du sentiment menace detecte par l IA texte (pondere par la confiance).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_weight_harassment", "label": "Scoring — poids harcelement (IA texte)", "type": "number", "required": false, "default": "7", "min": 0, "description": "Poids de base du sentiment harcelement detecte par l IA texte (pondere par la confiance).", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key": "score_weight_spam"}]'::jsonb);

-- Seuils d'action (baseline) ---------------------------------------------------
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "score_threshold_warn", "label": "Scoring — seuil warn", "type": "number", "required": false, "default": "2", "min": 0, "description": "Score total a partir duquel un avertissement est emis (baseline, si aucune regle per-flag ne s applique).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_threshold_delete", "label": "Scoring — seuil suppression", "type": "number", "required": false, "default": "4", "min": 0, "description": "Score total a partir duquel le message est supprime (baseline).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_threshold_mute", "label": "Scoring — seuil mute", "type": "number", "required": false, "default": "6", "min": 0, "description": "Score total a partir duquel l auteur est mute (baseline).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "score_threshold_ban", "label": "Scoring — seuil ban", "type": "number", "required": false, "default": "9", "min": 0, "description": "Score total a partir duquel l auteur est banni automatiquement (baseline).", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key": "score_threshold_warn"}]'::jsonb);
