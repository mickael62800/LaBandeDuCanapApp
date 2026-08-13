-- Ajoute le compteur de membres EN VOCAL au config_schema de welcome-bot.
--
-- Sur le modele du compteur de membres existant (counter_*), on renomme un
-- salon (idealement verrouille) avec le nombre de membres connectes en vocal :
-- "En Vocal : {count}". La config est stockee dans bot_guild_config et lue par
-- le bot via gRPC (WelcomeService.GetConfig). Trois nouvelles cles :
--   voice_counter_enabled / voice_counter_channel_id / voice_counter_format

UPDATE bot_definitions
SET config_schema = '[
  {"key": "welcome_enabled", "label": "Message de bienvenue", "type": "boolean", "required": false, "default": "true", "description": "Envoyer un embed de bienvenue dans le salon configure quand un nouveau membre rejoint."},
  {"key": "welcome_channel_id", "label": "Salon de bienvenue", "type": "channel", "required": false, "description": "Salon ou est poste le message de bienvenue."},
  {"key": "welcome_message", "label": "Message de bienvenue", "type": "text", "required": false, "default": "Bienvenue {user} sur **{server}** ! Tu es le **{count}e** membre.", "description": "Variables : {user}, {server}, {count}."},
  {"key": "welcome_embed_color", "label": "Couleur embed (hex)", "type": "text", "required": false, "default": "3498db", "description": "Code couleur hex sans # (ex: 3498db)."},
  {"key": "welcome_dm_enabled", "label": "DM de bienvenue", "type": "boolean", "required": false, "default": "false", "description": "Envoyer aussi un message prive au nouveau membre."},
  {"key": "welcome_dm_message", "label": "Message DM de bienvenue", "type": "text", "required": false, "default": "Bienvenue sur **{server}** ! N oublie pas de lire les regles.", "description": "Variables : {user}, {server}, {count}."},
  {"key": "rejoin_message", "label": "Message retour (rejoin)", "type": "text", "required": false, "default": "Content de te revoir {user} ! Tu nous avais manque.", "description": "Message affiche quand un membre deja connu re-rejoint le serveur."},
  {"key": "leave_enabled", "label": "Message de depart", "type": "boolean", "required": false, "default": "true", "description": "Envoyer un embed quand un membre quitte le serveur."},
  {"key": "leave_channel_id", "label": "Salon de depart", "type": "channel", "required": false},
  {"key": "leave_message", "label": "Message de depart", "type": "text", "required": false, "default": "{user} nous a quittes. Nous sommes maintenant **{count}** membres.", "description": "Variables : {user}, {server}, {count}."},
  {"key": "rules_enabled", "label": "Validation du reglement", "type": "boolean", "required": false, "default": "false", "description": "Afficher un bouton d acceptation du reglement qui attribue un role."},
  {"key": "rules_channel_id", "label": "Salon du reglement", "type": "channel", "required": false},
  {"key": "rules_message", "label": "Message du reglement", "type": "text", "required": false, "default": "Lis les regles et clique sur le bouton pour acceder au serveur."},
  {"key": "rules_role_id", "label": "Role apres validation", "type": "role", "required": false, "description": "Role attribue quand un membre clique sur le bouton d acceptation."},
  {"key": "rules_button_label", "label": "Libelle du bouton reglement", "type": "text", "required": false, "default": "J accepte les regles"},
  {"key": "counter_enabled", "label": "Compteur de membres", "type": "boolean", "required": false, "default": "false", "description": "Renomme un canal vocal avec le nombre de membres."},
  {"key": "counter_channel_id", "label": "Canal vocal compteur", "type": "channel", "required": false},
  {"key": "counter_format", "label": "Format compteur", "type": "text", "required": false, "default": "Membres : {count}", "description": "Variable : {count}."},
  {"key": "voice_counter_enabled", "label": "Compteur de membres en vocal", "type": "boolean", "required": false, "default": "false", "description": "Renomme un salon avec le nombre de membres actuellement connectes en vocal."},
  {"key": "voice_counter_channel_id", "label": "Salon compteur vocal", "type": "channel", "required": false},
  {"key": "voice_counter_format", "label": "Format compteur vocal", "type": "text", "required": false, "default": "En Vocal : {count}", "description": "Variable : {count}."},
  {"key": "anniversary_enabled", "label": "Anniversaires serveur", "type": "boolean", "required": false, "default": "false", "description": "Souhaiter un anniversaire d arrivee aux membres chaque annee."},
  {"key": "anniversary_channel_id", "label": "Salon anniversaires", "type": "channel", "required": false},
  {"key": "anniversary_message", "label": "Message anniversaire", "type": "text", "required": false, "default": "Felicitations {user}, ca fait **{years} an(s)** que tu es sur **{server}** !", "description": "Variables : {user}, {server}, {years}."}
]'::jsonb
WHERE bot_name = 'welcome-bot';
