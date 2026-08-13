-- Migration 153 : corrige les patches de descriptions de la 152 qui ne
-- matchaient pas les vraies cles des schemas (depuis renomme par les
-- migrations intermediaires).
--
-- Bots a 0/X dans le diagnostic post-152 :
--   audit-bot 0/12 -> structure changee par 114 (alert -> anomaly,
--                     join_leave / profile_edit ajoutes, log_retention removed)
--
-- On reutilise la fonction enrich_schema_keys creee en 152.

-- ══════════════════════════════════════════════════════════
-- audit-bot (vraies cles de la migration 114)
-- ══════════════════════════════════════════════════════════
SELECT enrich_schema_keys('audit-bot', '{
  "enabled": {"description": "Active ou desactive le module audit. Si OFF, plus aucun event Discord (joins, leaves, edits, deletes, role changes) n est logge."},
  "log_channel_id": {"description": "Salon de logs general (fallback). Recoit les events qui n ont pas de salon dedie configure ci-dessous."},
  "join_leave_channel_id": {"description": "Salon dedie aux entrees et sorties volontaires de membres. Si vide, fallback sur le salon de logs general."},
  "profile_edit_channel_id": {"description": "Salon dedie aux modifications de profil : changement de pseudo, avatar, ajout / retrait de roles, mute Discord."},
  "anomaly_channel_id": {"description": "Salon des alertes d urgence : mass-ban, mass-kick, mass-delete, mass-role detectes par anomaly_enabled. A surveiller en priorite."},
  "weekly_report_channel_id": {"description": "Salon ou est poste le rapport hebdomadaire automatique (chaque lundi). Vide = pas de rapport."},
  "message_cache_size": {"unit": "messages", "min": 100, "max": 100000,
    "description": "Taille du cache messages pour pouvoir afficher le contenu d un message supprime apres coup. Plus gros = plus de RAM. Recommande : 10000."},
  "anomaly_enabled": {"description": "Active la detection d anomalies : si plusieurs evenements similaires arrivent en moins de 60s, une alerte est postee dans anomaly_channel_id."},
  "anomaly_mass_ban_threshold": {"unit": "bans/60s", "min": 2, "max": 100,
    "description": "Nombre de bans en 60s pour declencher une alerte mass-ban. Recommande : 5."},
  "anomaly_mass_delete_threshold": {"unit": "deletes/60s", "min": 5, "max": 500,
    "description": "Nombre de suppressions de messages en 60s pour declencher une alerte mass-delete. Recommande : 20."},
  "anomaly_mass_role_threshold": {"unit": "changes/60s", "min": 5, "max": 200,
    "description": "Nombre de changements de roles en 60s pour declencher une alerte mass-role. Recommande : 10."},
  "weekly_report_enabled": {"description": "Genere un rapport hebdomadaire automatique (chaque lundi) avec les stats audit de la semaine. Necessite weekly_report_channel_id."}
}'::jsonb);
