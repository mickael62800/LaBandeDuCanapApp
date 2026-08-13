-- Enrichissement des configs pour tous les bots avec peu de parametres

-- ── Analytics Worker (3 → 10) ──
UPDATE bot_definitions SET config_schema = '[
    {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true"},
    {"key": "daily_snapshot_interval", "label": "Intervalle snapshot journalier (secondes)", "type": "number", "required": false, "default": "86400"},
    {"key": "hourly_snapshot_interval", "label": "Intervalle snapshot horaire (secondes)", "type": "number", "required": false, "default": "3600"},
    {"key": "data_retention_days", "label": "Retention des donnees (jours, 0 = illimite)", "type": "number", "required": false, "default": "90"},
    {"key": "monthly_report_enabled", "label": "Rapport mensuel automatique", "type": "boolean", "required": false, "default": "true"},
    {"key": "monthly_report_channel_id", "label": "Salon pour le rapport mensuel", "type": "channel", "required": false},
    {"key": "export_format", "label": "Format d export (json ou csv)", "type": "text", "required": false, "default": "json"},
    {"key": "top_users_count", "label": "Nombre d utilisateurs dans le top", "type": "number", "required": false, "default": "10"},
    {"key": "track_voice_stats", "label": "Tracker les stats vocales", "type": "boolean", "required": false, "default": "true"},
    {"key": "track_message_stats", "label": "Tracker les stats messages", "type": "boolean", "required": false, "default": "true"}
]' WHERE bot_name = 'analytics-worker';

-- ── Moderation Worker (4 → 12) ──
UPDATE bot_definitions SET config_schema = '[
    {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true"},
    {"key": "conduct_regen_interval", "label": "Intervalle regeneration points conduite (secondes)", "type": "number", "required": false, "default": "86400"},
    {"key": "ban_cleanup_interval", "label": "Intervalle nettoyage bans expires (secondes)", "type": "number", "required": false, "default": "3600"},
    {"key": "sync_ban_proposals_interval", "label": "Intervalle sync propositions ban (secondes)", "type": "number", "required": false, "default": "300"},
    {"key": "auto_escalation_enabled", "label": "Escalation automatique activee", "type": "boolean", "required": false, "default": "true"},
    {"key": "escalation_warn_to_mute", "label": "Nombre de warns avant auto-mute (0 = desactive)", "type": "number", "required": false, "default": "3"},
    {"key": "escalation_mute_to_ban", "label": "Nombre de mutes avant auto-ban (0 = desactive)", "type": "number", "required": false, "default": "3"},
    {"key": "default_temp_ban_duration_secs", "label": "Duree ban temporaire par defaut (secondes)", "type": "number", "required": false, "default": "86400"},
    {"key": "default_temp_mute_duration_secs", "label": "Duree mute temporaire par defaut (secondes)", "type": "number", "required": false, "default": "3600"},
    {"key": "notification_channel_id", "label": "Salon notifications escalation", "type": "channel", "required": false},
    {"key": "conduct_regen_amount", "label": "Points de conduite regeneres par cycle", "type": "number", "required": false, "default": "5"},
    {"key": "conduct_regen_max", "label": "Points de conduite maximum", "type": "number", "required": false, "default": "100"}
]' WHERE bot_name = 'moderation-worker';

-- ── Audit Bot (8 → 20) ──
UPDATE bot_definitions SET config_schema = '[
    {"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"},
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
    {"key": "alert_channel_id", "label": "Salon alertes urgentes", "type": "channel", "required": false},
    {"key": "message_cache_size", "label": "Taille cache messages", "type": "number", "required": false, "default": "10000"},
    {"key": "anomaly_enabled", "label": "Detection d anomalies", "type": "boolean", "required": false, "default": "true"},
    {"key": "anomaly_mass_ban_threshold", "label": "Seuil mass ban (en 60s)", "type": "number", "required": false, "default": "5"},
    {"key": "anomaly_mass_delete_threshold", "label": "Seuil mass delete (en 60s)", "type": "number", "required": false, "default": "20"},
    {"key": "anomaly_mass_role_threshold", "label": "Seuil mass role change (en 60s)", "type": "number", "required": false, "default": "10"},
    {"key": "weekly_report_enabled", "label": "Rapport hebdomadaire", "type": "boolean", "required": false, "default": "true"},
    {"key": "weekly_report_channel_id", "label": "Salon pour le rapport hebdomadaire", "type": "channel", "required": false},
    {"key": "log_retention_days", "label": "Retention des logs (jours, 0 = illimite)", "type": "number", "required": false, "default": "90"},
    {"key": "diff_permissions_enabled", "label": "Afficher le diff des permissions", "type": "boolean", "required": false, "default": "true"},
    {"key": "auto_archive_days", "label": "Archiver salons inactifs apres X jours (0 = desactive)", "type": "number", "required": false, "default": "0"},
    {"key": "archive_category_id", "label": "Categorie pour les archives", "type": "channel", "required": false},
    {"key": "log_message_edits", "label": "Logger les modifications de messages", "type": "boolean", "required": false, "default": "true"},
    {"key": "log_message_deletes", "label": "Logger les suppressions de messages", "type": "boolean", "required": false, "default": "true"},
    {"key": "log_member_changes", "label": "Logger les changements de membres", "type": "boolean", "required": false, "default": "true"},
    {"key": "log_role_changes", "label": "Logger les changements de roles", "type": "boolean", "required": false, "default": "true"},
    {"key": "log_voice_events", "label": "Logger les evenements vocaux", "type": "boolean", "required": false, "default": "true"},
    {"key": "ignored_channels", "label": "Salons ignores (IDs separes par des virgules)", "type": "text", "required": false}
]' WHERE bot_name = 'audit-bot';

-- ── Progression Bot (9 → 22) ──
UPDATE bot_definitions SET config_schema = '[
    {"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"},
    {"key": "tracking_enabled", "label": "Tracking XP actif", "type": "boolean", "required": false, "default": "true"},
    {"key": "leaderboard_default_size", "label": "Taille du leaderboard par defaut", "type": "number", "required": false, "default": "10"},
    {"key": "xp_cooldown_secs", "label": "Cooldown XP entre messages (secondes)", "type": "number", "required": false, "default": "60"},
    {"key": "xp_per_message", "label": "XP par message", "type": "number", "required": false, "default": "15"},
    {"key": "xp_per_voice_minute", "label": "XP par minute en vocal", "type": "number", "required": false, "default": "5"},
    {"key": "xp_channel_multipliers", "label": "Multiplicateurs XP par salon (salon_id:mult, CSV)", "type": "text", "required": false},
    {"key": "xp_role_multipliers", "label": "Multiplicateurs XP par role (role_id:mult, CSV)", "type": "text", "required": false},
    {"key": "levelup_channel_id", "label": "Salon annonces level-up", "type": "channel", "required": false},
    {"key": "levelup_message", "label": "Message level-up personnalise ({user} {level})", "type": "text", "required": false, "default": "Bravo {user} ! Tu as atteint le niveau {level} !"},
    {"key": "levelup_dm_enabled", "label": "Envoyer le level-up en DM", "type": "boolean", "required": false, "default": "false"},
    {"key": "levelup_announce_enabled", "label": "Annoncer les level-up dans le salon", "type": "boolean", "required": false, "default": "true"},
    {"key": "weekly_recap_enabled", "label": "Recap hebdomadaire", "type": "boolean", "required": false, "default": "true"},
    {"key": "streak_enabled", "label": "Systeme de streaks", "type": "boolean", "required": false, "default": "true"},
    {"key": "streak_bonus_xp", "label": "Bonus XP par jour de streak", "type": "number", "required": false, "default": "10"},
    {"key": "badges_enabled", "label": "Systeme de badges", "type": "boolean", "required": false, "default": "true"},
    {"key": "min_message_length", "label": "Longueur min du message pour gagner de l XP", "type": "number", "required": false, "default": "3"},
    {"key": "ignored_channels", "label": "Salons sans XP (IDs separes par des virgules)", "type": "text", "required": false},
    {"key": "ignored_roles", "label": "Roles sans XP (IDs separes par des virgules)", "type": "text", "required": false},
    {"key": "double_xp_roles", "label": "Roles avec double XP (IDs separes par des virgules)", "type": "text", "required": false},
    {"key": "max_level", "label": "Niveau maximum (0 = illimite)", "type": "number", "required": false, "default": "0"},
    {"key": "reset_on_leave", "label": "Reset XP quand le membre quitte", "type": "boolean", "required": false, "default": "false"}
]' WHERE bot_name = 'progression-bot';

-- ── Moderation Bot (14 → 26) ──
UPDATE bot_definitions SET config_schema = '[
    {"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"},
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
    {"key": "default_mute_duration_secs", "label": "Duree mute par defaut (secondes)", "type": "number", "required": false, "default": "600"},
    {"key": "max_mute_duration_secs", "label": "Duree max du mute (secondes)", "type": "number", "required": false, "default": "2419200"},
    {"key": "ban_delete_message_days", "label": "Jours de messages supprimes au ban", "type": "number", "required": false, "default": "1"},
    {"key": "warn_threshold_to_mute", "label": "Warns avant auto-mute (0 = desactive)", "type": "number", "required": false, "default": "0"},
    {"key": "dm_on_warn", "label": "Envoyer un DM a l utilisateur averti", "type": "boolean", "required": false, "default": "true"},
    {"key": "dm_on_mute", "label": "Envoyer un DM a l utilisateur mute", "type": "boolean", "required": false, "default": "true"},
    {"key": "dm_on_ban", "label": "Envoyer un DM a l utilisateur banni", "type": "boolean", "required": false, "default": "true"},
    {"key": "dm_on_kick", "label": "Envoyer un DM a l utilisateur expulse", "type": "boolean", "required": false, "default": "true"},
    {"key": "dm_warn_message", "label": "Message DM avertissement ({reason} {server})", "type": "text", "required": false, "default": "Vous avez recu un avertissement sur {server}. Raison : {reason}"},
    {"key": "dm_mute_message", "label": "Message DM mute ({reason} {server} {duration})", "type": "text", "required": false, "default": "Vous avez ete mute sur {server} pour {duration}. Raison : {reason}"},
    {"key": "dm_ban_message", "label": "Message DM ban ({reason} {server})", "type": "text", "required": false, "default": "Vous avez ete banni de {server}. Raison : {reason}"},
    {"key": "appeal_enabled", "label": "Lien d appel dans le DM de ban", "type": "boolean", "required": false, "default": "false"},
    {"key": "appeal_url", "label": "URL du formulaire d appel", "type": "text", "required": false},
    {"key": "color_warn", "label": "Couleur embed avertissement (hex sans #)", "type": "text", "required": false, "default": "f59e0b"},
    {"key": "color_mute", "label": "Couleur embed mute (hex sans #)", "type": "text", "required": false, "default": "ef4444"},
    {"key": "color_ban", "label": "Couleur embed ban (hex sans #)", "type": "text", "required": false, "default": "dc2626"},
    {"key": "color_kick", "label": "Couleur embed kick (hex sans #)", "type": "text", "required": false, "default": "f97316"},
    {"key": "color_unmute", "label": "Couleur embed unmute (hex sans #)", "type": "text", "required": false, "default": "2ecc71"},
    {"key": "color_unban", "label": "Couleur embed unban (hex sans #)", "type": "text", "required": false, "default": "2ecc71"},
    {"key": "show_avatar_in_logs", "label": "Afficher l avatar dans les logs", "type": "boolean", "required": false, "default": "true"},
    {"key": "confirm_ban", "label": "Demander confirmation avant ban", "type": "boolean", "required": false, "default": "true"},
    {"key": "confirm_kick", "label": "Demander confirmation avant kick", "type": "boolean", "required": false, "default": "false"},
    {"key": "ignored_roles", "label": "Roles immunises (IDs separes par des virgules)", "type": "text", "required": false},
    {"key": "notes_enabled", "label": "Systeme de notes sur les utilisateurs", "type": "boolean", "required": false, "default": "true"}
]' WHERE bot_name = 'moderation-bot';
