-- AutoMod : synchronise le config_schema des installations existantes.
--
-- Les champs de scoring (poids par flag + seuils warn/delete/mute/ban) et de
-- « tension de salon » ont ete ajoutes au schema DANS 001_init APRES que
-- certaines bases avaient deja ete initialisees. 001_init ne se rejoue pas :
-- ces bases affichent donc un panneau AutoMod SANS les champs de poids.
--
-- Cette migration les ajoute au schema UNIQUEMENT s'ils manquent (garde
-- idempotente sur score_weight_insult). Aucune valeur de config guild n'est
-- touchee : on ne modifie que la DEFINITION editable dans Composants.

UPDATE bot_definitions SET
    config_schema = config_schema || '[
        {"key": "channel_tension_enabled", "type": "boolean", "label": "Tension de salon activee", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active la detection d escalade par somme glissante des scores IA sur les N derniers messages d un salon."},
        {"key": "channel_tension_buffer_size", "type": "number", "label": "Taille du buffer glissant", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre de derniers messages d un salon inclus dans le calcul de tension."},
        {"key": "channel_tension_threshold_warn", "type": "number", "label": "Seuil tension - Warn", "default": "3.0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Somme cumulee des scores IA a partir de laquelle un warning est emis (0 pour desactiver ce palier)."},
        {"key": "channel_tension_threshold_delete", "type": "number", "label": "Seuil tension - Delete", "default": "5.0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Somme cumulee des scores IA a partir de laquelle le dernier message est supprime (0 pour desactiver)."},
        {"key": "channel_tension_threshold_mute", "type": "number", "label": "Seuil tension - Mute", "default": "7.0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Somme cumulee des scores IA a partir de laquelle le dernier auteur est mute (0 pour desactiver)."},
        {"key": "channel_tension_mute_duration_secs", "type": "number", "label": "Duree du mute tension (secondes)", "default": "300", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Duree du mute declenche par la tension de salon."},
        {"key": "channel_tension_warning_channel_id", "type": "channel", "label": "Salon de notification tension", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon ou poster les alertes de tension. Si vide, le message est poste dans le salon courant."},
        {"key": "score_weight_spam", "min": 0, "type": "number", "label": "Scoring — poids spam", "default": "3", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand un message est detecte comme spam."},
        {"key": "score_weight_insult", "min": 0, "type": "number", "label": "Scoring — poids insulte", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand un message contient une insulte."},
        {"key": "score_weight_link", "min": 0, "type": "number", "label": "Scoring — poids lien", "default": "1", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand un message contient un lien."},
        {"key": "score_weight_phishing", "min": 0, "type": "number", "label": "Scoring — poids phishing", "default": "7", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand un message est detecte comme phishing."},
        {"key": "score_weight_nsfw", "min": 0, "type": "number", "label": "Scoring — poids NSFW (image)", "default": "8", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand une image est classee NSFW par la vision IA."},
        {"key": "score_weight_illicit", "min": 0, "type": "number", "label": "Scoring — poids illicite (image)", "default": "9", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand une image est classee illicite par la vision IA."},
        {"key": "score_weight_anger", "min": 0, "type": "number", "label": "Scoring — poids colere (IA texte)", "default": "3", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids de base du sentiment colere detecte par l IA texte (pondere par la confiance)."},
        {"key": "score_weight_rage", "min": 0, "type": "number", "label": "Scoring — poids rage (IA texte)", "default": "6", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids de base du sentiment rage detecte par l IA texte (pondere par la confiance)."},
        {"key": "score_weight_threat", "min": 0, "type": "number", "label": "Scoring — poids menace (IA texte)", "default": "8", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids de base du sentiment menace detecte par l IA texte (pondere par la confiance)."},
        {"key": "score_weight_harassment", "min": 0, "type": "number", "label": "Scoring — poids harcelement (IA texte)", "default": "7", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids de base du sentiment harcelement detecte par l IA texte (pondere par la confiance)."},
        {"key": "score_threshold_warn", "min": 0, "type": "number", "label": "Scoring — seuil warn", "default": "2", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Score total a partir duquel un avertissement est emis (baseline, si aucune regle per-flag ne s applique)."},
        {"key": "score_threshold_delete", "min": 0, "type": "number", "label": "Scoring — seuil suppression", "default": "4", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Score total a partir duquel le message est supprime (baseline)."},
        {"key": "score_threshold_mute", "min": 0, "type": "number", "label": "Scoring — seuil mute", "default": "6", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Score total a partir duquel l auteur est mute (baseline)."},
        {"key": "score_threshold_ban", "min": 0, "type": "number", "label": "Scoring — seuil ban", "default": "9", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Score total a partir duquel l auteur est banni automatiquement (baseline)."}
    ]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key": "score_weight_insult"}]'::jsonb);
