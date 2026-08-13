-- ticket-bot — refonte du schema apres audit complet.
--
-- Schema 056 avait 22 cles. Toutes lues par le code SAUF :
--   - sla_first_response_minutes : pas de job qui le consomme
--   - transcript_format : pas de format alterne implemente
--
-- satisfaction_enabled etait dead avant ce commit (build_survey_message
-- jamais appele). Maintenant cable dans close.rs (commit accompagnant).
--
-- Adds depends_on cascade pour griser les sous-options quand le module
-- est OFF, ainsi que pour transcript_format/sla_first_response_minutes
-- avec marker TODO dans la description.

UPDATE bot_definitions SET
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active le systeme de tickets de support / appels de sanction."},

        {"key": "assistance_channel_id", "label": "Salon assistance", "type": "channel", "required": true, "description": "Salon ou le panneau d ouverture de ticket est poste.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "ticket_category_id", "label": "Categorie tickets", "type": "channel", "required": false, "description": "Categorie Discord ou les salons tickets sont crees. Vide = pas de categorie.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "admin_role_id", "label": "Role Administrateur", "type": "role", "required": true, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "moderator_role_id", "label": "Role Moderateur", "type": "role", "required": true, "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "max_open_per_user", "label": "Max tickets ouverts par user", "type": "number", "required": false, "default": "0", "min": 0, "max": 50, "description": "0 = illimite. Au-dela, le user ne peut plus en ouvrir.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "inactive_close_days", "label": "Fermeture auto si inactif", "type": "number", "required": false, "default": "7", "min": 0, "max": 90, "unit": "j", "description": "0 = desactive. Tickets sans activite > N jours sont fermes par le worker.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "close_delay_secs", "label": "Delai avant suppression salon", "type": "number", "required": false, "default": "5", "min": 0, "max": 600, "unit": "s", "description": "Apres validation de fermeture, on attend N secondes avant de delete le salon.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "welcome_message", "label": "Message d accueil custom", "type": "text", "required": false, "default": "", "description": "Vide = message par defaut. Affiche dans le salon ticket a sa creation.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "transcript_dm_enabled", "label": "Transcript en DM a la fermeture", "type": "boolean", "required": false, "default": "true", "description": "Envoie le transcript du ticket en DM a son auteur a la fermeture.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "transcript_format", "label": "Format transcript", "type": "enum", "required": false, "default": "text", "options": [{"value": "text", "label": "Texte simple"}, {"value": "markdown", "label": "Markdown"}, {"value": "html", "label": "HTML (TODO)"}], "description": "TODO : seul text est cable aujourd hui. Les autres formats sont prevus.", "depends_on": {"key": "transcript_dm_enabled", "equals": "true"}},

        {"key": "satisfaction_enabled", "label": "Sondage satisfaction (1-5 etoiles)", "type": "boolean", "required": false, "default": "true", "description": "A la fermeture, envoie un sondage avec 5 boutons etoiles dans le DM transcript. Necessite transcript_dm_enabled.", "depends_on": {"key": "transcript_dm_enabled", "equals": "true"}},

        {"key": "sla_escalation_minutes", "label": "Delai escalade SLA appels", "type": "number", "required": false, "default": "60", "min": 0, "max": 1440, "unit": "min", "description": "0 = desactive. Tickets de type \"appel_sanction\" sans premiere reponse > N minutes sont escalades par le worker.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "sla_first_response_minutes", "label": "SLA premiere reponse", "type": "number", "required": false, "default": "30", "min": 0, "max": 1440, "unit": "min", "description": "TODO : pas encore cable (alerte si pas de premiere reponse > N minutes).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "appeal_sla_scan_interval", "label": "Worker : intervalle scan SLA appels", "type": "number", "required": false, "default": "300", "min": 30, "max": 3600, "unit": "s", "description": "Frequence de scan des tickets d appel pour detection de depassement SLA.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "response_templates", "label": "Templates de reponses", "type": "text", "required": false, "default": "", "description": "Format CSV multilignes : label|contenu (un par ligne). Disponibles via /ticket reponse.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "faq_entries", "label": "FAQ", "type": "text", "required": false, "default": "", "description": "Format CSV multilignes : question|reponse (une par ligne). Affiche dans /faq.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "color_normal", "label": "Couleur ticket normal (hex)", "type": "text", "required": false, "default": "2ecc71", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "color_urgent", "label": "Couleur ticket urgent (hex)", "type": "text", "required": false, "default": "ff6600", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "color_confidential", "label": "Couleur ticket confidentiel (hex)", "type": "text", "required": false, "default": "e74c3c", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "color_staff", "label": "Couleur embed staff (hex)", "type": "text", "required": false, "default": "e67e22", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "color_user", "label": "Couleur embed user (hex)", "type": "text", "required": false, "default": "3498db", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'ticket-bot';
