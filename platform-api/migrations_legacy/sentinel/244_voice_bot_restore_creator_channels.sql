-- voice-bot — restauration des salons "lobby" creators dans le schema.
--
-- Regression introduite par 239_voice_bot_complete_schema.sql : cette
-- migration a supprime public/private/game_creator_channel_id du
-- config_schema en les croyant "env-only". C'est FAUX : le code les lit
-- bien depuis bot_guild_config (member_events.rs:46-60 via
-- get_guild_config_for), l'env n'etant qu'un fallback.
--
-- Consequence : la page Composants (pilotee par config_schema) n'affichait
-- plus les champs pour choisir les salons lobby public/prive/jeu, rendant
-- impossible la configuration de la creation de salons temporaires via
-- l'interface web.
--
-- log_channel_id n'est PAS restaure : lui est reellement env-only (lu
-- depuis ConfigKey/env dans config.rs + embeds.rs, jamais depuis la DB).
--
-- On reecrit le schema complet = schema de 239 + les 3 salons creators.

UPDATE bot_definitions SET
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active le systeme de salons vocaux temporaires (lobby create -> join -> salon perso)."},

        {"key": "public_creator_channel_id", "label": "Lobby salon public", "type": "channel", "required": false, "description": "Salon vocal lobby : le rejoindre cree un nouveau salon temporaire public.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "private_creator_channel_id", "label": "Lobby salon prive", "type": "channel", "required": false, "description": "Salon vocal lobby : le rejoindre cree un salon prive (acces sur invitation).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "game_creator_channel_id", "label": "Lobby salon de jeu", "type": "channel", "required": false, "description": "Salon vocal lobby : le rejoindre cree un salon de jeu (categorie dediee). Laisser vide pour desactiver.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "afk_enabled", "label": "AFK sweep actif", "type": "boolean", "required": false, "default": "false", "description": "Tache periodique qui deplace/kick les membres AFK (self_mute + self_deaf trop longtemps).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "afk_timeout_minutes", "label": "Delai AFK", "type": "number", "required": false, "default": "10", "min": 1, "max": 1440, "unit": "min", "description": "Apres combien de minutes en self_mute + self_deaf un membre est considere AFK.", "depends_on": {"key": "afk_enabled", "equals": "true"}},
        {"key": "afk_channel_id", "label": "Salon AFK", "type": "channel", "required": false, "description": "Salon vocal ou les membres AFK sont deplaces.", "depends_on": {"key": "afk_enabled", "equals": "true"}},
        {"key": "afk_move_owner", "label": "Deplacer aussi les owners", "type": "boolean", "required": false, "default": "false", "description": "Si OFF, le proprietaire d un salon temporaire ne sera jamais deplace en AFK (evite que le salon se ferme).", "depends_on": {"key": "afk_enabled", "equals": "true"}},

        {"key": "voice_creation_cooldown_secs", "label": "Cooldown creation salon", "type": "number", "required": false, "default": "5", "min": 0, "max": 600, "unit": "s", "description": "Delai minimum entre 2 creations de salon par un meme user (anti-spam).", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "voice_empty_cleanup_delay_secs", "label": "Delai suppression salon vide", "type": "number", "required": false, "default": "2", "min": 0, "max": 60, "unit": "s", "description": "Anti-race : on attend N secondes avant de supprimer un salon vide (le owner peut revenir vite).", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "voice_flood_max_messages", "label": "Seuil flood (messages)", "type": "number", "required": false, "default": "5", "min": 1, "max": 50, "description": "Nombre de clics panel admin dans la fenetre avant mute auto.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "voice_flood_time_window_secs", "label": "Fenetre flood", "type": "number", "required": false, "default": "5", "min": 1, "max": 60, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "voice_flood_mute_duration_secs", "label": "Duree mute si flood", "type": "number", "required": false, "default": "30", "min": 30, "max": 3600, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "voice_vote_kick_timeout_secs", "label": "Duree vote-kick", "type": "number", "required": false, "default": "60", "min": 30, "max": 600, "unit": "s", "description": "Apres ce delai, le vote-kick expire automatiquement (sans verdict).", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'voice-bot';
