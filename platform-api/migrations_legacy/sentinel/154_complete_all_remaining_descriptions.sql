-- Migration 154 : complete les descriptions de TOUTES les cles encore sans
-- description (143 cles identifiees apres la 152/153). Reutilise la fonction
-- enrich_schema_keys creee en 152.

-- ══════════════════════════════════════════════════════════
-- blackjack-bot (10 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('blackjack-bot', '{
  "allow_double_down": {"description": "Autoriser le double down (doubler la mise apres 2 cartes initiales)."},
  "blackjack_payout": {"unit": "x", "min": 1, "max": 10,
    "description": "Multiplicateur de gain en cas de blackjack naturel (As + 10 sur 2 cartes). Standard : 1.5x."},
  "category_blackjack": {"description": "Categorie Discord ou les tables blackjack sont creees."},
  "channel_blackjack": {"description": "Salon principal du blackjack (creation de tables, leaderboards)."},
  "cooldown_secs": {"unit": "secondes", "min": 0, "max": 3600,
    "description": "Cooldown entre 2 parties pour un meme joueur. 0 = pas de cooldown."},
  "log_channel_id": {"description": "Salon ou sont loggees les parties (gains/pertes, blackjacks naturels)."},
  "max_daily_games": {"unit": "parties/jour", "min": 0, "max": 1000,
    "description": "Nombre max de parties qu un joueur peut faire par jour. 0 = illimite."},
  "max_players_per_table": {"unit": "joueurs", "min": 1, "max": 7,
    "description": "Nombre max de joueurs par table (limite Discord/UX). Standard : 4-5."},
  "shoe_decks": {"unit": "decks", "min": 1, "max": 8,
    "description": "Nombre de jeux de cartes dans le sabot (shoe). Plus haut = moins de comptage de cartes possible. Standard casino : 6-8."},
  "starting_coins": {"unit": "coins", "min": 0, "max": 1000000,
    "description": "Coins de depart pour un nouveau joueur blackjack."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- cache-worker (3 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('cache-worker', '{
  "analytics_cache_refresh": {"unit": "minutes", "min": 1, "max": 60,
    "description": "Frequence du warm-up du cache Redis analytics. Recommande : 5."},
  "dashboard_cache_refresh": {"unit": "minutes", "min": 1, "max": 60,
    "description": "Frequence du warm-up du cache dashboard. Recommande : 5."},
  "voice_stats_cache_refresh": {"unit": "minutes", "min": 1, "max": 60,
    "description": "Frequence du warm-up des stats vocales en cache. Recommande : 5."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- cleanup-worker (7 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('cleanup-worker', '{
  "enabled": {"description": "Active ou desactive le worker. Si OFF, aucun cleanup automatique des vieilles donnees."},
  "cleanup_interval_hours": {"unit": "heures", "min": 1, "max": 168,
    "description": "Frequence du cleanup des vieilles donnees. Recommande : 24 (1 fois par jour)."},
  "closed_tickets_retention_days": {"unit": "jours", "min": 0, "max": 3650,
    "description": "Nombre de jours pendant lesquels on garde les tickets fermes en DB. 0 = illimite."},
  "logs_retention_days": {"unit": "jours", "min": 0, "max": 365,
    "description": "Retention des logs applicatifs (table logs). Au-dela, les anciens logs sont purges. 0 = illimite. Recommande : 30."},
  "vacuum_enabled": {"description": "Active le VACUUM PostgreSQL automatique sur les tables hot. Recupere l espace disque apres les DELETE."},
  "vacuum_interval_hours": {"unit": "heures", "min": 1, "max": 168,
    "description": "Frequence du VACUUM. Recommande : 24."},
  "voice_sessions_retention_days": {"unit": "jours", "min": 0, "max": 365,
    "description": "Nombre de jours pendant lesquels on garde l historique des sessions vocales. 0 = illimite."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- coude-bot (13 cles manquantes)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('coude-bot', '{
  "class_change_cooldown_days": {"unit": "jours", "min": 0, "max": 365,
    "description": "Cooldown entre 2 changements de classe pour un meme joueur."},
  "class_change_cost": {"unit": "coins", "min": 0, "max": 1000000,
    "description": "Cout en coins pour changer de classe."},
  "combat_max_rounds": {"unit": "rounds", "min": 1, "max": 50,
    "description": "Nombre max de tours dans un combat avant declaration d egalite forcee."},
  "hp_min_combat_pct": {"unit": "%", "min": 0, "max": 100,
    "description": "% de HP minimum requis pour pouvoir lancer un combat. Empeche les combats avec quasi 0 HP."},
  "hp_regen_per_hour": {"unit": "HP/h", "min": 0, "max": 10000,
    "description": "Quantite de HP regeneres par heure (taux de base). Modifie par les paliers ci-dessous."},
  "hp_regen_rate_0_25": {"unit": "x", "min": 0, "max": 5,
    "description": "Multiplicateur de regen HP quand le joueur est entre 0% et 25% de ses HP max. Plus haut = regen plus rapide a bas HP."},
  "hp_regen_rate_25_50": {"unit": "x", "min": 0, "max": 5,
    "description": "Multiplicateur de regen HP entre 25% et 50% de HP."},
  "hp_regen_rate_50_75": {"unit": "x", "min": 0, "max": 5,
    "description": "Multiplicateur de regen HP entre 50% et 75% de HP."},
  "hp_regen_rate_75_100": {"unit": "x", "min": 0, "max": 5,
    "description": "Multiplicateur de regen HP entre 75% et 100% de HP."},
  "repos_cooldown_hours": {"unit": "heures", "min": 0, "max": 168,
    "description": "Cooldown de la commande /repos (regen forcee de HP). Empeche le farm."},
  "reset_stats_cost": {"unit": "coins", "min": 0, "max": 10000000,
    "description": "Cout en coins pour reset ses stats (compteurs combats / heists / etc.)."},
  "season_duration_days": {"unit": "jours", "min": 1, "max": 365,
    "description": "Duree d une saison Coup de Coude. A la fin, les stats sont archivees et les leaderboards reset."},
  "steal_max_daily": {"unit": "vols/jour", "min": 0, "max": 100,
    "description": "Nombre max de vols qu un joueur peut tenter par jour. 0 = illimite."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- coude-worker (2 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('coude-worker', '{
  "combat_expiry_check_secs": {"unit": "secondes", "min": 10, "max": 3600,
    "description": "Frequence du scan des combats en attente d expiration. Recommande : 60."},
  "combat_expiry_hours": {"unit": "heures", "min": 1, "max": 168,
    "description": "Apres combien d heures un combat pending non accepte est marque comme expire."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- game-bot (3 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('game-bot', '{
  "log_channel_id": {"description": "Salon ou sont logges les events des mini-jeux (gagnants, scores)."},
  "max_games": {"unit": "parties", "min": 1, "max": 100,
    "description": "Nombre max de parties simultanees toutes confondues."},
  "role_color": {"description": "Couleur (hex) des roles attribues aux gagnants. Format : 0xFFA500 ou FFA500."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- image-bot (9 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('image-bot', '{
  "channel_thresholds": {"description": "Seuils de detection par salon. Format : channel_id:threshold,channel_id:threshold (CSV). Override le confidence_threshold global."},
  "confidence_threshold": {"unit": "0..1", "min": 0, "max": 1,
    "description": "Confidence min de l IA vision pour declencher une action. Recommande : 0.5. Plus bas = plus sensible mais plus de faux positifs."},
  "hash_cache_enabled": {"description": "Active le cache des hash d images analysees (evite re-analyse de la meme image)."},
  "hash_cache_ttl_secs": {"unit": "secondes", "min": 60, "max": 2592000,
    "description": "Duree de validite d un hash en cache. Recommande : 86400 (1 jour)."},
  "ignored_roles": {"description": "IDs de roles dont les membres ne sont pas analyses (mods, trusted), separes par virgules."},
  "max_image_size_mb": {"unit": "Mo", "min": 1, "max": 25,
    "description": "Taille max d une image analysee. Au-dela, skip. Discord limite a 25Mo en upload normal."},
  "queue_enabled": {"description": "Active la file d attente async via ai-worker (POST /api/ai/jobs). Sinon analyse synchrone bloquante."},
  "queue_max_retries": {"unit": "tentatives", "min": 0, "max": 10,
    "description": "Nombre max de retries pour un job IA en echec avant abandon."},
  "scan_embeds": {"description": "Analyse aussi les images dans les embeds (liens preview), pas seulement les pieces jointes."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- moderation-bot (23 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('moderation-bot', '{
  "appeal_enabled": {"description": "Active la possibilite pour un user sanctionne de faire appel via /appeal."},
  "appeal_url": {"description": "URL externe optionnelle pour les appels (formulaire Google, page custom). Vide = appel via DM uniquement."},
  "ban_delete_message_days": {"unit": "jours", "min": 0, "max": 7,
    "description": "Lors d un ban, supprime aussi les messages des X derniers jours du banni. Max Discord : 7."},
  "color_ban": {"description": "Couleur (hex) de l embed pour un ban. Ex: ED4245 (rouge)."},
  "color_kick": {"description": "Couleur (hex) de l embed pour un kick. Ex: FAA61A (orange)."},
  "color_mute": {"description": "Couleur (hex) de l embed pour un mute / timeout."},
  "color_unban": {"description": "Couleur (hex) de l embed pour un unban (vert habituellement)."},
  "color_unmute": {"description": "Couleur (hex) de l embed pour un unmute."},
  "color_warn": {"description": "Couleur (hex) de l embed pour un warn."},
  "confirm_ban": {"description": "Demande une confirmation avant d executer un /ban (anti fat-finger)."},
  "confirm_kick": {"description": "Demande une confirmation avant d executer un /kick."},
  "dm_ban_message": {"description": "Texte du DM envoye au membre banni. Variables : {user}, {server}, {reason}, {duration}."},
  "dm_mute_message": {"description": "Texte du DM envoye au membre mute. Variables : {user}, {server}, {reason}, {duration}."},
  "dm_on_ban": {"description": "Envoie un DM au membre lors d un ban."},
  "dm_on_kick": {"description": "Envoie un DM au membre lors d un kick (avant qu il ne soit kicke pour qu il le recoive)."},
  "dm_on_mute": {"description": "Envoie un DM au membre lors d un mute."},
  "dm_on_warn": {"description": "Envoie un DM au membre lors d un warn."},
  "dm_warn_message": {"description": "Texte du DM envoye au membre averti. Variables : {user}, {server}, {reason}, {gravity}."},
  "ignored_roles": {"description": "IDs de roles dont les membres ne peuvent pas etre sanctionnes par /ban /mute etc. (admins, autres mods). Separes par virgules."},
  "max_mute_duration_secs": {"unit": "secondes", "min": 60, "max": 2419200,
    "description": "Duree max autorisee dans /mute (au-dela, refuse). Max Discord : 2419200 (28 jours)."},
  "notes_enabled": {"description": "Active la commande /note (notes privees mod sur un user)."},
  "show_avatar_in_logs": {"description": "Affiche l avatar du sanctionne dans l embed de log."},
  "warn_threshold_to_mute": {"unit": "warns", "min": 0, "max": 20,
    "description": "Nombre de warns actifs avant suggestion de mute auto. 0 = desactive (utilise plutot moderation-worker)."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- monitoring-worker (2 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('monitoring-worker', '{
  "enabled": {"description": "Active ou desactive le worker monitoring (detection offline/online des bots via heartbeats Redis)."},
  "check_interval": {"unit": "secondes", "min": 5, "max": 600,
    "description": "Frequence du check des heartbeats. Recommande : 30."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- progression-bot (15 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('progression-bot', '{
  "badges_enabled": {"description": "Active le systeme de badges (recompenses palieres : 100 messages, 1000 XP, etc.)."},
  "double_xp_roles": {"description": "Roles qui recoivent x2 XP. IDs separes par virgules."},
  "ignored_channels": {"description": "Salons ou les messages ne donnent pas d XP (spam, off-topic). IDs separes par virgules."},
  "ignored_roles": {"description": "Roles dont les membres ne gagnent pas d XP (bots, admins). IDs separes par virgules."},
  "leaderboard_default_size": {"unit": "users", "min": 5, "max": 100,
    "description": "Nombre d utilisateurs affiches par defaut dans /leaderboard."},
  "levelup_announce_enabled": {"description": "Annonce les level-ups dans levelup_channel_id (sinon dans le salon courant)."},
  "levelup_channel_id": {"description": "Salon ou sont annonces les level-ups si levelup_announce_enabled est ON."},
  "levelup_dm_enabled": {"description": "Envoie aussi un DM au membre lors d un level-up."},
  "levelup_message": {"description": "Message de level-up custom. Variables : {user}, {level}."},
  "max_level": {"unit": "niveau", "min": 0, "max": 1000,
    "description": "Niveau maximum atteignable. 0 = illimite. Au-dela, plus de level-up."},
  "min_message_length": {"unit": "caracteres", "min": 0, "max": 200,
    "description": "Longueur min d un message pour donner de l XP (anti-flood 1 lettre)."},
  "reset_on_leave": {"description": "Reset l XP du membre quand il quitte le serveur (vs garde si il revient)."},
  "streak_bonus_xp": {"unit": "XP/jour", "min": 0, "max": 10000,
    "description": "Bonus XP attribue par jour de streak active. Cumulable jusqu a un palier."},
  "tracking_enabled": {"description": "Active le tracking XP. Si OFF, plus aucun XP n est attribue."},
  "weekly_recap_enabled": {"description": "Genere un recap hebdomadaire des progressions chaque lundi."},
  "xp_channel_multipliers": {"description": "Multiplicateurs XP par salon. Format CSV : salon_id:mult,salon_id:mult (ex: 12345:2,67890:0.5)."},
  "xp_role_multipliers": {"description": "Multiplicateurs XP par role. Format CSV : role_id:mult,role_id:mult."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- security-bot (10 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('security-bot', '{
  "alt_name_distance": {"unit": "edits", "min": 0, "max": 20,
    "description": "Distance de Levenshtein max entre 2 pseudos pour les considerer comme alts potentiels. Plus bas = plus strict. Standard : 3."},
  "alt_retention_secs": {"unit": "secondes", "min": 3600, "max": 31536000,
    "description": "Duree de retention des fingerprints d alts en DB. Au-dela, supprimes. Recommande : 2592000 (30 jours)."},
  "captcha_type": {"type": "enum", "options": [
      {"value": "math", "label": "Calcul mental"},
      {"value": "image", "label": "Image avec bruit"},
      {"value": "button", "label": "Simple bouton de verification"}
    ],
    "description": "Type de captcha pour les nouveaux membres. button = simplest, image = anti-bot le plus efficace."},
  "lockdown_duration_secs": {"unit": "secondes", "min": 60, "max": 86400,
    "description": "Duree par defaut d un lockdown automatique declenche par anti-raid."},
  "min_account_age_secs": {"unit": "secondes", "min": 0, "max": 31536000,
    "description": "Age minimum d un compte Discord pour etre autorise a join sans suspicion. Recommande : 604800 (7 jours)."},
  "quarantine_enabled": {"description": "Active le systeme de quarantaine (role @Quarantaine pour comptes suspects)."},
  "raid_join_window_secs": {"unit": "secondes", "min": 5, "max": 600,
    "description": "Fenetre temporelle pour compter les joins suspectes. Combine avec raid_join_threshold."},
  "raid_pattern_enabled": {"description": "Active la detection de patterns de raid avances (pseudos similaires, avatars identiques, dates de creation tres proches)."},
  "raid_pattern_score_threshold": {"unit": "score", "min": 1, "max": 100,
    "description": "Score min de pattern raid pour declencher une alerte. Plus bas = plus sensible. Recommande : 50."},
  "slowmode_seconds": {"unit": "secondes", "min": 0, "max": 21600,
    "description": "Slowmode applique automatiquement sur les salons concernes en cas de raid detecte. 0 = pas de slowmode."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- ticket-bot (19 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('ticket-bot', '{
  "admin_role_id": {"description": "Role admin qui a acces a tous les tickets (au-dessus du staff)."},
  "assistance_channel_id": {"description": "Salon ou les users postent leur demande initiale (commande /ticket ou bouton)."},
  "close_delay_secs": {"unit": "secondes", "min": 0, "max": 3600,
    "description": "Delai avant suppression definitive du salon de ticket apres la fermeture. Permet de relire avant suppression."},
  "color_confidential": {"description": "Couleur (hex) des embeds pour tickets confidentiels."},
  "color_normal": {"description": "Couleur (hex) des embeds pour tickets normaux."},
  "color_staff": {"description": "Couleur (hex) des messages staff dans un ticket."},
  "color_urgent": {"description": "Couleur (hex) des tickets marques urgents."},
  "color_user": {"description": "Couleur (hex) des messages user dans un ticket."},
  "faq_entries": {"description": "Entrees FAQ proposees automatiquement aux users. Format JSON ou CSV de question:reponse."},
  "inactive_close_days": {"unit": "jours", "min": 1, "max": 90,
    "description": "Apres combien de jours sans message un ticket est ferme automatiquement pour inactivite."},
  "max_open_per_user": {"unit": "tickets", "min": 1, "max": 10,
    "description": "Nombre max de tickets ouverts simultanement par un meme user."},
  "moderator_role_id": {"description": "Role moderateur (acces tickets standards, sauf confidentiels)."},
  "response_templates": {"description": "Templates de reponses rapides pour le staff. Format JSON ou CSV cle:texte."},
  "satisfaction_enabled": {"description": "Envoie un sondage de satisfaction (1-5 etoiles) au user apres fermeture du ticket."},
  "sla_escalation_minutes": {"unit": "minutes", "min": 5, "max": 1440,
    "description": "Delai apres lequel un ticket sans reponse staff est escalade aux admins."},
  "sla_first_response_minutes": {"unit": "minutes", "min": 1, "max": 1440,
    "description": "Delai max pour la premiere reponse staff. Au-dela : warning SLA."},
  "transcript_dm_enabled": {"description": "Envoie automatiquement le transcript en DM au user apres fermeture."},
  "transcript_format": {"type": "enum", "options": [
      {"value": "html", "label": "HTML (lisible)"},
      {"value": "txt", "label": "Texte brut"},
      {"value": "json", "label": "JSON (structure)"}
    ],
    "description": "Format du transcript genere a la fermeture."},
  "welcome_message": {"description": "Message d accueil dans un nouveau ticket. Variables : {user}, {ticket_id}."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- voice-bot (19 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('voice-bot', '{
  "afk_enabled": {"description": "Active la detection AFK (membre en self_mute + self_deaf trop longtemps)."},
  "afk_move_owner": {"description": "Si ON, l owner du salon peut aussi etre deplace en AFK. Sinon, immune."},
  "afk_timeout_minutes": {"unit": "minutes", "min": 1, "max": 1440,
    "description": "Apres combien de minutes en self_mute + self_deaf un membre est considere AFK."},
  "auto_delete_empty": {"description": "Supprime automatiquement les salons temporaires vides apres empty_check_delay_secs."},
  "color_created": {"description": "Couleur (hex) de l embed lors de la creation d un salon temp."},
  "color_deleted": {"description": "Couleur (hex) de l embed lors de la suppression d un salon temp."},
  "color_joined": {"description": "Couleur (hex) de l embed lors d un join dans un salon temp."},
  "color_left": {"description": "Couleur (hex) de l embed lors d un leave d un salon temp."},
  "cooldown_secs": {"unit": "secondes", "min": 0, "max": 3600,
    "description": "Cooldown entre 2 creations de salon par le meme user (anti-spam de creation)."},
  "default_channel_name": {"description": "Format du nom par defaut des nouveaux salons temp. Variables : {user}, {count}."},
  "default_member_limit": {"unit": "users", "min": 0, "max": 99,
    "description": "Limite de places par defaut sur un nouveau salon (0 = illimite)."},
  "empty_check_delay_secs": {"unit": "secondes", "min": 5, "max": 3600,
    "description": "Apres combien de secondes un salon vide est verifie pour suppression. Court delai = suppression rapide."},
  "flood_max_messages": {"unit": "messages", "min": 2, "max": 50,
    "description": "Nombre de messages dans le panel admin du vocal avant declencher un flood mute."},
  "flood_window_secs": {"unit": "secondes", "min": 1, "max": 300,
    "description": "Fenetre temporelle pour le comptage des messages flood sur le panel admin."},
  "queue_enabled_by_default": {"description": "Si ON, les nouveaux salons prives ont la file d attente activee par defaut. Sinon, acces libre via invitation."},
  "queue_user_limit": {"unit": "users", "min": 1, "max": 100,
    "description": "Taille max de la file d attente d un salon prive."},
  "vote_majority_percent": {"unit": "%", "min": 50, "max": 100,
    "description": "% de votes Yes requis pour qu un vote-kick passe. Recommande : 60-75."},
  "vote_min_members": {"unit": "users", "min": 2, "max": 20,
    "description": "Nombre min de membres dans le vocal pour declencher un vote-kick."},
  "vote_timeout_secs": {"unit": "secondes", "min": 30, "max": 600,
    "description": "Duree d un vote-kick avant cloture automatique."}
}'::jsonb);

-- ══════════════════════════════════════════════════════════
-- welcome-bot (6 cles)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('welcome-bot', '{
  "anniversary_channel_id": {"description": "Salon ou sont publies les messages d anniversaire d arrivee. Vide = desactive."},
  "counter_channel_id": {"description": "Salon vocal renomme avec le compteur de membres (ex: \"Membres : 1234\")."},
  "leave_channel_id": {"description": "Salon ou est poste le message de depart quand un membre quitte. Vide = utilise welcome_channel_id."},
  "rules_button_label": {"description": "Texte du bouton de validation du reglement (ex: \"J accepte\")."},
  "rules_channel_id": {"description": "Salon ou est poste le message de reglement avec le bouton de validation."},
  "rules_message": {"description": "Message du reglement affiche dans le rules_channel_id. Markdown supporte."}
}'::jsonb);
