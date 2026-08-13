-- Enrichit le config_schema d'analytics-worker avec :
--   * description (texte d'aide affiche a droite de l'input)
--   * unit + min + max (clamp pour empecher les valeurs aberrantes)
--   * options (pour les enums comme export_format)
--
-- Contexte : un admin a saisi 86400 dans daily_snapshot_interval (en pensant
-- "secondes") alors que le code attend des heures, ce qui a multiplie
-- l'intervalle par 3600 -> 9.86 ans entre 2 runs. Resultat : aucun snapshot
-- pendant 6 jours. Cette migration corrige le schema pour que la web UI
-- affiche l'unite et clampe les valeurs hors borne.
--
-- IMPORTANT : on change aussi les types "text" -> "enum" pour export_format
-- pour avoir un dropdown au lieu d'un input texte libre.

UPDATE bot_definitions SET config_schema = '[
    {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true",
     "description": "Active ou desactive completement le worker. Si OFF, aucun snapshot n est ecrit et les graphiques deviennent vides."},

    {"key": "daily_snapshot_interval", "label": "Intervalle snapshot quotidien", "type": "number", "required": false, "default": "1",
     "unit": "heures", "min": 1, "max": 168,
     "description": "Frequence du recalcul de la ligne du jour dans daily_activity. 1h donne un dashboard a jour heure par heure. Recommande : 1."},

    {"key": "hourly_snapshot_interval", "label": "Intervalle snapshot horaire", "type": "number", "required": false, "default": "60",
     "unit": "minutes", "min": 5, "max": 1440,
     "description": "Frequence du snapshot par tranche horaire (utilise pour la heatmap activite). Recommande : 60."},

    {"key": "data_retention_days", "label": "Retention des donnees", "type": "number", "required": false, "default": "90",
     "unit": "jours", "min": 0, "max": 3650,
     "description": "Nombre de jours d historique conserves dans daily_activity / hourly_activity. 0 = illimite. 90 jours suffit pour les graphiques sur 30 / 90 jours."},

    {"key": "monthly_report_enabled", "label": "Rapport mensuel automatique", "type": "boolean", "required": false, "default": "false",
     "description": "Si ON, le worker poste un recap mensuel le 1er du mois dans le salon ci-dessous."},

    {"key": "monthly_report_channel_id", "label": "Salon du rapport mensuel", "type": "channel", "required": false,
     "description": "Salon ou le recap mensuel sera publie. Laisse vide si le rapport mensuel est desactive."},

    {"key": "export_format", "label": "Format d export", "type": "enum", "required": false, "default": "json",
     "options": [
       {"value": "json", "label": "JSON"},
       {"value": "csv", "label": "CSV"}
     ],
     "description": "Format des exports analytics demandes via l API. JSON pour integrations, CSV pour Excel."},

    {"key": "top_users_count", "label": "Taille du top utilisateurs", "type": "number", "required": false, "default": "10",
     "unit": "utilisateurs", "min": 1, "max": 100,
     "description": "Nombre d utilisateurs affiches dans le classement du dashboard."},

    {"key": "track_voice_stats", "label": "Tracker les stats vocales", "type": "boolean", "required": false, "default": "true",
     "description": "Comptabilise les minutes en vocal de chaque membre. Si OFF, la colonne voice_minutes reste a 0."},

    {"key": "track_message_stats", "label": "Tracker les stats messages", "type": "boolean", "required": false, "default": "true",
     "description": "Comptabilise les messages de chaque membre. Si OFF, la colonne messages reste a 0 et les graphiques deviennent inutiles."}
]'::jsonb WHERE bot_name = 'analytics-worker';
