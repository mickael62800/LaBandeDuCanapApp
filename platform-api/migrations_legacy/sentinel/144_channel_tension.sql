-- Ajoute les cles de configuration pour le systeme de "tension de salon"
-- (somme glissante des scores IA des N derniers messages d'un salon).
-- Si le cumul depasse un seuil, declenche Warn/Delete/Mute en plus de
-- l'analyse individuelle.
--
-- Cles ajoutees au schema de `automod-bot` :
--   * channel_tension_enabled            (bool, default false)
--   * channel_tension_buffer_size        (number, default 5)
--   * channel_tension_threshold_warn     (number, default 3.0)
--   * channel_tension_threshold_delete   (number, default 5.0)
--   * channel_tension_threshold_mute     (number, default 7.0)
--   * channel_tension_mute_duration_secs (number, default 300)
--   * channel_tension_warning_channel_id (channel, optionnel)

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "channel_tension_enabled", "label": "Tension de salon activee", "type": "boolean", "required": false, "default": "false", "description": "Active la detection d escalade par somme glissante des scores IA sur les N derniers messages d un salon."},
  {"key": "channel_tension_buffer_size", "label": "Taille du buffer glissant", "type": "number", "required": false, "default": "5", "description": "Nombre de derniers messages d un salon inclus dans le calcul de tension."},
  {"key": "channel_tension_threshold_warn", "label": "Seuil tension - Warn", "type": "number", "required": false, "default": "3.0", "description": "Somme cumulee des scores IA a partir de laquelle un warning est emis (0 pour desactiver ce palier)."},
  {"key": "channel_tension_threshold_delete", "label": "Seuil tension - Delete", "type": "number", "required": false, "default": "5.0", "description": "Somme cumulee des scores IA a partir de laquelle le dernier message est supprime (0 pour desactiver)."},
  {"key": "channel_tension_threshold_mute", "label": "Seuil tension - Mute", "type": "number", "required": false, "default": "7.0", "description": "Somme cumulee des scores IA a partir de laquelle le dernier auteur est mute (0 pour desactiver)."},
  {"key": "channel_tension_mute_duration_secs", "label": "Duree du mute tension (secondes)", "type": "number", "required": false, "default": "300", "description": "Duree du mute declenche par la tension de salon."},
  {"key": "channel_tension_warning_channel_id", "label": "Salon de notification tension", "type": "channel", "required": false, "default": "", "description": "Salon ou poster les alertes de tension. Si vide, le message est poste dans le salon courant."}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key": "channel_tension_enabled"}]'::jsonb);
