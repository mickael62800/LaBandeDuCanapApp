-- Enrichit le config_schema de TOUS les bots/workers avec :
--   * description : texte d aide pedagogique affiche a droite de l input
--                   (cf. ConfigFieldRow.vue dans sentinel-web).
--   * unit / min / max : pour les inputs number, evite les valeurs aberrantes
--                        (cf. bug daily_snapshot_interval = 86400 dans 151).
--   * options : pour les types text qu on convertit en enum (dropdown).
--
-- Strategie : fonction PL/pgSQL `enrich_schema_keys` qui PATCHE le schema
-- existant en ajoutant les champs description/unit/min/max/options sur les
-- entrees existantes (par cle), sans toucher aux autres champs. Idempotent.
-- N ecrase pas les descriptions deja presentes (UPDATE conditionnel).

CREATE OR REPLACE FUNCTION enrich_schema_keys(p_bot_name TEXT, p_patch JSONB)
RETURNS VOID AS $$
DECLARE
    v_schema JSONB;
    v_new_schema JSONB := '[]'::jsonb;
    v_entry JSONB;
    v_key TEXT;
    v_overrides JSONB;
BEGIN
    SELECT config_schema INTO v_schema FROM bot_definitions WHERE bot_name = p_bot_name;
    IF v_schema IS NULL THEN
        RAISE NOTICE 'enrich_schema_keys: bot % introuvable, skip', p_bot_name;
        RETURN;
    END IF;

    FOR v_entry IN SELECT * FROM jsonb_array_elements(v_schema)
    LOOP
        v_key := v_entry->>'key';
        v_overrides := p_patch->v_key;
        IF v_overrides IS NOT NULL THEN
            -- Merge : les champs du patch ecrasent ceux de l entree existante.
            v_entry := v_entry || v_overrides;
        END IF;
        v_new_schema := v_new_schema || jsonb_build_array(v_entry);
    END LOOP;

    UPDATE bot_definitions SET config_schema = v_new_schema WHERE bot_name = p_bot_name;
END;
$$ LANGUAGE plpgsql;


-- ══════════════════════════════════════════════════════════
-- automod-bot — auto-moderation textuelle / IA / vision
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('automod-bot', '{
  "enabled": {"description": "Active ou desactive completement l auto-moderation. Si OFF, aucun message n est analyse."},
  "log_channel_id": {"description": "Salon ou les actions automod sont loggees (suppressions, mutes, cartes de review). Indispensable si ai_review_mode = true."},
  "ai_review_mode": {"description": "Si ON, le bot envoie une carte de review au mod au lieu d agir directement. Tres utile en phase de tuning. Si OFF, l action est appliquee automatiquement."},
  "spam_detection_enabled": {"description": "Detecte les messages dupliques / floodes (regex)."},
  "insult_detection_enabled": {"description": "Detecte les insultes par regex (FR + EN, leet speak). Complementaire de l IA texte qui detecte plutot les sentiments (rage, menace)."},
  "insult_custom_words": {"description": "Liste de mots customs separes par des virgules. Detection case-insensitive, substring match."},
  "link_detection_enabled": {"description": "Detecte les liens HTTP. Combine avec phishing_detection pour scoring eleve."},
  "phishing_detection_enabled": {"description": "Detecte les domaines de phishing connus. Score eleve par defaut (7.0) -> action severe."},
  "caps_enabled": {"description": "Detecte l abus de majuscules (au-dela du seuil ci-dessous)."},
  "caps_threshold_chars": {"unit": "caracteres", "min": 5, "max": 500, "description": "A partir de combien de caracteres en majuscules le message est flag."},
  "flood_max_messages": {"unit": "messages", "min": 2, "max": 100, "description": "Nombre max de messages dans la fenetre flood avant warning + envoi a l IA."},
  "flood_window_secs": {"unit": "secondes", "min": 1, "max": 300, "description": "Fenetre temporelle pour le flood. Defaut : 10s pour 5 messages."},
  "mute_duration_secs": {"unit": "secondes", "min": 60, "max": 2419200, "description": "Duree du timeout Discord applique en cas d action mute. Max Discord : 28 jours."},
  "text_enabled": {"description": "Active l inference IA texte (DistilBERT) pour detecter rage / menace / harcelement / colere."},
  "text_threshold": {"unit": "0..1", "min": 0, "max": 1, "description": "Confidence minimale pour qu un flag IA soit actif. Plus bas = plus sensible. Recommande : 0.5. A baisser a 0.35 pour catcher des cas borderline."},
  "vision_enabled": {"description": "Active l inference IA vision (EfficientNet) sur les images jointes. Detecte NSFW et contenu illicite."},
  "vision_threshold": {"unit": "0..1", "min": 0, "max": 1, "description": "Confidence minimale pour qu un flag vision soit actif. Recommande : 0.5."},
  "context_dampening": {"unit": "0..1", "min": 0, "max": 1, "description": "Multiplicateur du score IA si du contexte conversationnel est present. 1.0 = pas d attenuation, 0.65 = score divise par 1.5 (defaut). Reduit les faux positifs entre potes."},
  "context_format": {"type": "enum", "options": [{"value": "natural", "label": "Naturel (texte brut)"}, {"value": "tagged", "label": "Balises [message]/[context]"}], "description": "Comment le contexte est formate pour l IA. natural = simple, tagged = balises explicites (peut ameliorer la qualite selon le modele)."},
  "context_max_messages": {"unit": "messages", "min": 0, "max": 20, "description": "Nombre de messages precedents inclus comme contexte. 0 = pas de contexte."},
  "context_max_chars": {"unit": "caracteres", "min": 50, "max": 1000, "description": "Longueur max de chaque message de contexte. Au-dela, tronque."},
  "files_review_mode": {"description": "Si ON, les fichiers suspects sont mis en review (pas supprimes auto)."},
  "suspicious_files_enabled": {"description": "Detecte les pieces jointes a extension dangereuse (.exe, .bat, .vbs, etc.)."},
  "flood_review_mode": {"description": "Si ON, le flood passe en carte de review au lieu de warner directement."},
  "caps_review_mode": {"description": "Si ON, l abus de majuscules passe en carte de review."},
  "night_mode_enabled": {"description": "Active des seuils plus stricts pendant les heures de nuit (ci-dessous)."},
  "night_start_hour": {"unit": "heure", "min": 0, "max": 23, "description": "Heure de debut du night mode (24h)."},
  "night_end_hour": {"unit": "heure", "min": 0, "max": 23, "description": "Heure de fin du night mode (24h)."},
  "ignored_channels": {"description": "IDs de salons exclus de l automod, separes par virgules."},
  "ignored_roles": {"description": "IDs de roles dont les membres sont exclus de l automod (mods, etc.), separes par virgules."},
  "adaptive_slowmode_enabled": {"description": "Active automatiquement le slowmode Discord si le salon depasse un seuil de messages."},
  "adaptive_slowmode_threshold": {"unit": "messages/min", "min": 1, "max": 200, "description": "Seuil de messages par minute pour declencher le slowmode."},
  "adaptive_slowmode_seconds": {"unit": "secondes", "min": 1, "max": 21600, "description": "Duree du slowmode applique."},
  "jackpot_threshold": {"unit": "coins", "min": 100, "max": 1000000, "description": "Montant minimum d un credit pour declencher un taunt jackpot (cf. coude). N a pas d effet si l economie n est pas active."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- moderation-bot — sanctions manuelles (slash commands)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('moderation-bot', '{
  "enabled": {"description": "Active ou desactive le module moderation (commandes /ban /mute /warn etc.)."},
  "log_channel_id": {"description": "Salon ou sont logguees les sanctions appliquees."},
  "appeal_channel_id": {"description": "Salon ou les utilisateurs peuvent faire appel d une sanction."},
  "default_warn_gravity": {"type": "enum", "options": [{"value": "low", "label": "Faible"}, {"value": "medium", "label": "Moyenne"}, {"value": "high", "label": "Haute"}], "description": "Gravite par defaut quand un mod fait /warn sans la specifier."},
  "default_mute_duration_secs": {"unit": "secondes", "min": 60, "max": 2419200, "description": "Duree du mute si le mod ne specifie pas de duree dans /mute. Max Discord : 28 jours."},
  "default_ban_duration_secs": {"unit": "secondes", "min": 0, "max": 31536000, "description": "Duree d un ban temporaire par defaut. 0 = ban permanent."},
  "dm_on_sanction": {"description": "Envoie un DM au membre sanctionne avec le motif et la duree."},
  "templates_enabled": {"description": "Active les templates de raisons rapides (/template apply)."},
  "review_required_for": {"description": "Liste des actions qui passent en review obligatoire avant application (csv : ban,mute,kick)."},
  "auto_archive_appeals_days": {"unit": "jours", "min": 1, "max": 365, "description": "Apres combien de jours un appel non resolu est archive automatiquement."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- moderation-worker — escalations + cleanup bans + rappels
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('moderation-worker', '{
  "enabled": {"description": "Active ou desactive le worker. Si OFF : pas d escalation auto, pas de cleanup bans, pas de rappels expiration."},
  "conduct_regen_interval": {"unit": "heures", "min": 1, "max": 168, "description": "Frequence de regeneration des points de conduite. Recommande : 24 (1 fois par jour)."},
  "ban_cleanup_interval": {"unit": "minutes", "min": 1, "max": 60, "description": "Frequence du scan des bans expires. Recommande : 1 (toutes les minutes pour reactivite)."},
  "sync_ban_proposals_interval": {"unit": "minutes", "min": 1, "max": 30, "description": "Frequence de sync des propositions de ban en attente."},
  "auto_escalation_enabled": {"description": "Si ON, le bot escalade automatiquement (warn -> mute -> ban) selon les seuils ci-dessous."},
  "escalation_warn_to_mute": {"unit": "warns", "min": 0, "max": 20, "description": "Nombre de warns actifs avant qu un nouveau warn declenche un mute auto. 0 = desactive."},
  "escalation_mute_to_ban": {"unit": "mutes", "min": 0, "max": 20, "description": "Nombre de mutes actifs avant qu un nouveau mute declenche un ban auto. 0 = desactive."},
  "default_temp_ban_duration_secs": {"unit": "secondes", "min": 3600, "max": 31536000, "description": "Duree du ban auto declenche par escalation. Defaut : 86400 (1 jour)."},
  "default_temp_mute_duration_secs": {"unit": "secondes", "min": 60, "max": 2419200, "description": "Duree du mute auto declenche par escalation. Max Discord : 28 jours."},
  "notification_channel_id": {"description": "Salon ou les escalations auto sont notifiees."},
  "conduct_regen_amount": {"unit": "points", "min": 1, "max": 100, "description": "Combien de points de conduite sont regeneres par cycle."},
  "conduct_regen_max": {"unit": "points", "min": 10, "max": 1000, "description": "Plafond du score de conduite (pas de regen au-dela)."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- security-bot — anti-raid, alts, captcha, lockdown
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('security-bot', '{
  "enabled": {"description": "Active ou desactive le module security."},
  "log_channel_id": {"description": "Salon ou les events security (raids detectes, alts, lockdowns) sont logges."},
  "alert_channel_id": {"description": "Salon des alertes urgentes (raid en cours, mass-ban detecte). Peut etre identique au log."},
  "raid_detection_enabled": {"description": "Detecte les vagues de joins suspectes (raid)."},
  "raid_join_threshold": {"unit": "joins", "min": 2, "max": 100, "description": "Nombre de joins dans la fenetre raid_window pour declencher le raid mode."},
  "raid_window_secs": {"unit": "secondes", "min": 5, "max": 600, "description": "Fenetre temporelle pour compter les joins."},
  "captcha_enabled": {"description": "Force les nouveaux membres a passer un captcha avant d acceder au serveur."},
  "captcha_role_id": {"description": "Role attribue avant validation du captcha (= membre restreint)."},
  "alt_detection_enabled": {"description": "Detecte les comptes alternatifs (meme IP, fingerprint similaire)."},
  "alt_min_account_age_days": {"unit": "jours", "min": 0, "max": 365, "description": "Ages minimum d un compte Discord pour etre considere comme non-alt. En dessous, suspicion levee."},
  "lockdown_enabled": {"description": "Permet d activer le mode lockdown (nouveaux membres mutes auto) via /security lockdown."},
  "lockdown_role_id": {"description": "Role attribue aux nouveaux membres en lockdown."},
  "quarantine_role_id": {"description": "Role de quarantaine (acces ultra-restreint) pour comptes suspects."},
  "ban_threshold": {"unit": "events", "min": 1, "max": 50, "description": "Nombre d events de securite avant ban auto d un user (alt, multi-account)."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- voice-bot — salons vocaux temporaires
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('voice-bot', '{
  "enabled": {"description": "Active ou desactive le module salons vocaux dynamiques."},
  "public_creator_channel_id": {"description": "Salon vocal lobby : rejoindre cree un nouveau salon temporaire public."},
  "private_creator_channel_id": {"description": "Salon vocal lobby pour creer un salon prive (acces sur invitation)."},
  "game_creator_channel_id": {"description": "Salon vocal lobby pour creer un salon de jeu (categorie dediee)."},
  "log_channel_id": {"description": "Salon ou sont logges les events voice (creation, suppression, transferts)."},
  "delete_empty_after_secs": {"unit": "secondes", "min": 10, "max": 3600, "description": "Apres combien de secondes un salon vide est automatiquement supprime."},
  "afk_timeout_secs": {"unit": "secondes", "min": 60, "max": 86400, "description": "Apres combien de secondes en self_mute + self_deaf un membre est marque AFK."},
  "afk_action": {"type": "enum", "options": [{"value": "none", "label": "Aucune"}, {"value": "move", "label": "Deplacer en salon AFK"}, {"value": "kick", "label": "Kicker du vocal"}], "description": "Action prise quand un membre est detecte AFK."},
  "afk_channel_id": {"description": "Salon AFK ou les membres inactifs sont deplaces si afk_action = move."},
  "max_channels_per_user": {"unit": "salons", "min": 1, "max": 10, "description": "Nombre max de salons temporaires qu un meme user peut posseder en simultane."},
  "default_user_limit": {"unit": "users", "min": 0, "max": 99, "description": "Limite de places par defaut sur un nouveau salon (0 = illimite)."},
  "vote_kick_threshold_pct": {"unit": "%", "min": 50, "max": 100, "description": "% de votes Yes requis pour qu un vote-kick passe. Recommande : 75."},
  "flood_mute_duration_secs": {"unit": "secondes", "min": 30, "max": 3600, "description": "Duree du mute auto si un membre flood le panel admin."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- audit-bot — logs d audit Discord + anomaly detection
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('audit-bot', '{
  "enabled": {"description": "Active ou desactive le module audit."},
  "log_channel_id": {"description": "Salon principal des logs audit (joins, leaves, messages supprimes, role changes)."},
  "alert_channel_id": {"description": "Salon des alertes urgentes (mass-ban, mass-delete detectes)."},
  "message_cache_size": {"unit": "messages", "min": 100, "max": 100000, "description": "Taille du cache messages pour pouvoir afficher le contenu d un message supprime."},
  "anomaly_enabled": {"description": "Active la detection d anomalies (mass-ban, mass-delete, mass-role-change en peu de temps)."},
  "anomaly_mass_ban_threshold": {"unit": "bans/60s", "min": 2, "max": 100, "description": "Nombre de bans en 60s pour declencher une alerte mass-ban."},
  "anomaly_mass_delete_threshold": {"unit": "deletes/60s", "min": 5, "max": 500, "description": "Nombre de deletes en 60s pour declencher une alerte mass-delete."},
  "anomaly_mass_role_threshold": {"unit": "changes/60s", "min": 5, "max": 200, "description": "Nombre de changements de roles en 60s pour declencher une alerte."},
  "weekly_report_enabled": {"description": "Genere un rapport hebdomadaire automatique (chaque lundi)."},
  "weekly_report_channel_id": {"description": "Salon ou est poste le rapport hebdo. Vide = desactive."},
  "log_retention_days": {"unit": "jours", "min": 0, "max": 365, "description": "Nombre de jours de retention des logs audit en DB. 0 = illimite."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- progression-bot — XP, levels, badges
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('progression-bot', '{
  "enabled": {"description": "Active ou desactive le module XP / niveaux."},
  "level_up_channel_id": {"description": "Salon ou est annonce un level-up. Vide = annonce dans le salon courant."},
  "xp_per_message": {"unit": "XP", "min": 0, "max": 1000, "description": "XP gagnes par message envoye."},
  "xp_cooldown_secs": {"unit": "secondes", "min": 0, "max": 3600, "description": "Cooldown entre 2 gains d XP par message (anti-farm). Recommande : 60."},
  "xp_per_voice_minute": {"unit": "XP/min", "min": 0, "max": 100, "description": "XP gagnes par minute en vocal actif."},
  "xp_voice_min_users": {"unit": "users", "min": 1, "max": 10, "description": "Nombre min de users dans le vocal pour que l XP voice soit comptabilise (anti-AFK alone)."},
  "default_role_id": {"description": "Roles attribues par defaut a tous les nouveaux membres (ou apres rules). Multi-roles supportes : separes par virgules."},
  "level_role_rewards": {"description": "Roles attribues a certains paliers. Format : level:role_id,level:role_id (ex: 5:111,10:222)."},
  "streak_enabled": {"description": "Tracke les streaks de connexion quotidienne."},
  "streak_reward_xp": {"unit": "XP", "min": 0, "max": 10000, "description": "Bonus XP attribue lors d une streak (atteinte d un palier)."},
  "leaderboard_size": {"unit": "users", "min": 5, "max": 100, "description": "Nombre d utilisateurs affiches dans /leaderboard."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- coude-bot — jeu Coup de Coude
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('coude-bot', '{
  "enabled": {"description": "Active ou desactive le jeu Coup de Coude."},
  "log_channel_id": {"description": "Salon ou sont logges les combats / heists / events."},
  "starting_coins": {"unit": "coins", "min": 0, "max": 1000000, "description": "Coins de depart pour un nouveau joueur."},
  "starting_hp": {"unit": "HP", "min": 1, "max": 10000, "description": "HP de depart pour un nouveau joueur."},
  "combat_min_mise": {"unit": "coins", "min": 0, "max": 100000, "description": "Mise minimale acceptee pour un combat."},
  "combat_max_mise": {"unit": "coins", "min": 0, "max": 10000000, "description": "Mise maximale acceptee pour un combat."},
  "combat_pending_expiry_hours": {"unit": "heures", "min": 1, "max": 168, "description": "Apres combien d heures un combat en pending non accepte expire."},
  "betting_window_secs": {"unit": "secondes", "min": 30, "max": 3600, "description": "Duree de la phase de paris avant resolution du combat."},
  "betting_min_amount": {"unit": "coins", "min": 0, "max": 100000, "description": "Mise minimale d un pari."},
  "heist_cooldown_days": {"unit": "jours", "min": 0, "max": 30, "description": "Cooldown entre 2 braquages par le meme joueur."},
  "heist_prison_hours": {"unit": "heures", "min": 0, "max": 168, "description": "Duree d emprisonnement en cas d echec de braquage."},
  "heist_gain_min_percent": {"unit": "%", "min": 0, "max": 100, "description": "% min de la cagnotte gagne lors d un braquage reussi."},
  "heist_gain_max_percent": {"unit": "%", "min": 0, "max": 100, "description": "% max de la cagnotte gagne lors d un braquage reussi."},
  "tournament_prize_pool_percent": {"unit": "%", "min": 0, "max": 100, "description": "% de la cagnotte hebdo distribue en prix de tournoi."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- ticket-bot — systeme de tickets support
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('ticket-bot', '{
  "enabled": {"description": "Active ou desactive le module tickets."},
  "ticket_category_id": {"description": "Categorie Discord ou sont crees les salons de ticket."},
  "log_channel_id": {"description": "Salon ou sont logges les events ticket (creation, fermeture, transcripts)."},
  "staff_role_id": {"description": "Role staff qui a acces a tous les tickets."},
  "max_tickets_per_user": {"unit": "tickets", "min": 1, "max": 10, "description": "Nombre max de tickets ouverts simultanement par un meme user."},
  "auto_close_hours": {"unit": "heures", "min": 1, "max": 720, "description": "Apres combien d heures sans message un ticket est ferme automatiquement."},
  "transcript_enabled": {"description": "Genere un transcript HTML a la fermeture du ticket."},
  "satisfaction_survey_enabled": {"description": "Envoie un sondage de satisfaction au user apres fermeture."},
  "sla_warning_minutes": {"unit": "minutes", "min": 1, "max": 1440, "description": "Apres combien de minutes sans reponse staff un ticket est en warning SLA."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- community-bot — roles temporaires, panels, parrainage
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('community-bot', '{
  "enabled": {"description": "Active ou desactive le module community."},
  "log_channel_id": {"description": "Salon ou sont logges les events community (parrainage, panels)."},
  "sponsor_enabled": {"description": "Active le systeme de parrainage."},
  "sponsor_reward_xp": {"unit": "XP", "min": 0, "max": 100000, "description": "XP attribue au parrain quand son filleul atteint le niveau cible."},
  "sponsor_target_level": {"unit": "level", "min": 1, "max": 100, "description": "Niveau que le filleul doit atteindre pour valider le parrainage."},
  "panels_enabled": {"description": "Active les panels reactionnels (auto-roles via reactions ou boutons)."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- blackjack-bot
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('blackjack-bot', '{
  "enabled": {"description": "Active ou desactive le module blackjack."},
  "table_channel_id": {"description": "Salon dedie aux tables blackjack persistantes."},
  "min_bet": {"unit": "coins", "min": 1, "max": 1000000, "description": "Mise minimale pour rejoindre une table."},
  "max_bet": {"unit": "coins", "min": 1, "max": 100000000, "description": "Mise maximale pour rejoindre une table."},
  "afk_timeout_secs": {"unit": "secondes", "min": 30, "max": 600, "description": "Apres combien de secondes d inactivite un joueur est considere AFK et eject de la table."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- game-bot — mini-jeux textuels
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('game-bot', '{
  "enabled": {"description": "Active ou desactive les mini-jeux."},
  "channel_id": {"description": "Salon dedie aux mini-jeux. Vide = autorise partout."},
  "emoji_host_guild_id": {"description": "ID de la guild qui heberge les emojis customs utilises dans les jeux."},
  "trivia_timeout_secs": {"unit": "secondes", "min": 5, "max": 300, "description": "Temps de reponse pour une question trivia."},
  "reward_xp": {"unit": "XP", "min": 0, "max": 10000, "description": "XP attribue aux gagnants."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- image-bot — analyse images NSFW/illicite
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('image-bot', '{
  "enabled": {"description": "Active ou desactive l analyse automatique des images."},
  "log_channel_id": {"description": "Salon ou sont logges les images detectees comme NSFW / illicit."},
  "auto_delete_nsfw": {"description": "Supprime automatiquement les images detectees NSFW."},
  "auto_delete_illicit": {"description": "Supprime automatiquement les images detectees comme contenu illicite."},
  "hash_cache_ttl_hours": {"unit": "heures", "min": 1, "max": 720, "description": "Duree pendant laquelle un hash d image est mis en cache (evite re-analyse)."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- cache-worker
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('cache-worker', '{
  "enabled": {"description": "Active ou desactive le worker. Si OFF, les vues materialisees ne sont plus refresh et les leaderboards stagnent."},
  "warm_cache_interval": {"unit": "minutes", "min": 1, "max": 60, "description": "Frequence du warm-up des caches Redis (analytics, dashboard)."},
  "mv_refresh_interval": {"unit": "minutes", "min": 1, "max": 60, "description": "Frequence du refresh des vues materialisees leaderboards. Recommande : 5."},
  "user_cache_sync_interval": {"unit": "minutes", "min": 5, "max": 120, "description": "Frequence du sync de la table user_cache (usernames Discord)."},
  "manage_partitions_interval": {"unit": "heures", "min": 1, "max": 168, "description": "Frequence de la gestion des partitions mensuelles (creation M+1, M+2). Recommande : 24."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- coude-worker — expirations + resolution paris
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('coude-worker', '{
  "enabled": {"description": "Active ou desactive le worker coude. Si OFF, les combats expires ne sont plus resolus et les paris en attente restent figes."},
  "expire_combats_interval": {"unit": "heures", "min": 1, "max": 24, "description": "Frequence du scan des combats pending expires. Recommande : 24."},
  "resolve_betting_interval": {"unit": "secondes", "min": 5, "max": 600, "description": "Frequence du scan des combats betting a resoudre. Recommande : 30."}
}'::jsonb);


-- ══════════════════════════════════════════════════════════
-- Cleanup : on garde la fonction enrich_schema_keys (utile pour les
-- prochaines migrations qui voudront enrichir d autres bots).
-- Pour la dropper proprement, decommentez la ligne suivante :
-- DROP FUNCTION enrich_schema_keys(TEXT, JSONB);
