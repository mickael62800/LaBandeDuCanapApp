-- Migration 051 : Ajoute le config_schema pour les features avancees du community-bot
-- (roles temporaires, exclusifs, prerequis, parrainage, booster)

UPDATE bot_definitions
SET config_schema = '[
  {"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"},
  {"key": "exclusive_groups", "label": "Groupes exclusifs (group:role1,role2 par ligne)", "type": "text", "required": false, "default": ""},
  {"key": "role_prerequisites", "label": "Prerequis roles (role_id:requires_role:other ou role_id:min_days:N par ligne)", "type": "text", "required": false, "default": ""},
  {"key": "temp_roles", "label": "Roles temporaires (role_id:duree_secs par ligne)", "type": "text", "required": false, "default": ""},
  {"key": "booster_role_id", "label": "Role attribue aux boosters", "type": "role", "required": false, "default": ""},
  {"key": "max_sponsorships", "label": "Max filleuls actifs par parrain", "type": "number", "required": false, "default": "3"},
  {"key": "seasonal_roles", "label": "Roles saisonniers (role_id:mois_debut-mois_fin par ligne)", "type": "text", "required": false, "default": ""}
]'::jsonb
WHERE bot_name = 'community-bot';
