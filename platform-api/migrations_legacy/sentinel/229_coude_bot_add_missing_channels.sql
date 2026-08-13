-- coude-bot — ajoute les 7 channels que le bot consomme mais qui
-- n'etaient pas exposes dans le schema UI.
--
-- Code reads (sentinel-bot/src/modules/coude/guild_config.rs) :
--   - channel_combats         : salon principal des combats
--   - channel_leaderboard     : ou poster le top players
--   - channel_profil          : ou poster les profils joueurs
--   - channel_activites       : salon ou les commandes activitees sont dispos
--   - channel_announcements   : annonces de saison, gros evenements
--   - channel_notifications   : notifications individuelles (defenseur attaque...)
--   - tournament_channel_id   : salon dedie aux tournois (fallback channel_activites)
--
-- Append au schema existant (jsonb concat). Idempotent : skip si deja
-- present (filtre via NOT EXISTS).

UPDATE bot_definitions SET
    config_schema = config_schema || '[
        {"key": "channel_combats", "label": "Salon combats", "type": "channel", "required": false, "description": "Salon principal ou les combats /coude se deroulent (embeds initial + rounds).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "channel_leaderboard", "label": "Salon leaderboard", "type": "channel", "required": false, "description": "Salon ou poster le top players. Si vide, /leaderboard repond dans le salon courant.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "channel_profil", "label": "Salon profils", "type": "channel", "required": false, "description": "Salon dedie pour /profil. Si vide, repond dans le salon courant.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "channel_activites", "label": "Salon activites", "type": "channel", "required": false, "description": "Salon principal du module (pari, tout-ou-rien, vol). Restriction : les commandes ne fonctionnent que dans ce salon si configure.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "channel_announcements", "label": "Salon annonces gameplay", "type": "channel", "required": false, "description": "Annonces de saison, gros evenements (chaos quotidien, prestige). Si vide, fallback log_channel_id.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "channel_notifications", "label": "Salon notifications", "type": "channel", "required": false, "description": "Notifications individuelles (vol initie, combat lance contre toi...). Si vide, DM au user.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "tournament_channel_id", "label": "Salon tournois", "type": "channel", "required": false, "description": "Salon dedie aux tournois. Si vide, fallback channel_activites.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "channel_combats"}]'::jsonb);
