-- Aligne les config_schema de bot_definitions sur ce que le code lit
-- reellement. Apres audit complet (avril 2026) modules + workers.
--
-- Resume des actions :
--   * Ajoute l'entree cleanup-bot (absente)
--   * Ajoute les entrees workers manquantes (ai, appeal-sla, audit-cache,
--     blackjack-cleanup, discord-audit-sync, export, temp-roles)
--   * Ajoute les cles manquantes : automod (+2), progression (+1),
--     coude-bot (+20), cache-worker (+3), coude-worker (+4), moderation-worker (+1)
--   * Nettoie : retire casino_* de coude-bot (code mort), retire
--     expiry_penalty_percent/refund_bets_on_expiry de coude-worker (dead)
--   * Deplace hp_regen_tick_secs de coude-bot vers coude-worker (mauvais endroit)

-- ══════════════════════════════════════════════════════════
-- 1. Nouveaux bot_definitions
-- ══════════════════════════════════════════════════════════

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema) VALUES
('cleanup-bot', 'Cleanup',
  'Commandes de nettoyage /purge et /cleanup',
  '[
    {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active ou desactive les commandes de nettoyage."}
  ]'::jsonb),

('ai-worker', 'Worker IA',
  'Traitement asynchrone des jobs IA (texte + vision)',
  '[
    {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true", "description": "Active ou desactive le worker IA."},
    {"key": "ai_poll_interval", "label": "Intervalle polling jobs IA (secondes)", "type": "number", "required": false, "default": "5", "description": "Frequence d extraction des jobs IA depuis la file ai_jobs."},
    {"key": "ai_job_timeout", "label": "Timeout job IA (secondes)", "type": "number", "required": false, "default": "60", "description": "Duree max d un job IA avant de le marquer failed."}
  ]'::jsonb),

('appeal-sla-worker', 'Worker SLA Appels',
  'Escalade des tickets d appel de sanction au-dela du SLA',
  '[
    {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true", "description": "Active ou desactive le worker d escalade SLA."},
    {"key": "appeal_sla_scan_interval", "label": "Intervalle scan SLA (secondes)", "type": "number", "required": false, "default": "300", "description": "Frequence de scan des tickets d appel pour detection de depassement SLA."}
  ]'::jsonb),

('audit-cache-worker', 'Worker Cache Audit',
  'Rafraichit periodiquement le cache Redis des audit logs',
  '[
    {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true", "description": "Active ou desactive le worker."},
    {"key": "audit_cache_refresh_interval", "label": "Rafraichissement cache audit (secondes)", "type": "number", "required": false, "default": "60", "description": "Frequence de regeneration du cache Redis des audit logs."}
  ]'::jsonb),

('blackjack-cleanup-worker', 'Worker Cleanup Blackjack',
  'Nettoyage periodique des parties de blackjack expirees',
  '[
    {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true", "description": "Active ou desactive le worker."},
    {"key": "blackjack_cleanup_scan_interval", "label": "Intervalle scan blackjack (secondes)", "type": "number", "required": false, "default": "60", "description": "Frequence de nettoyage des parties blackjack expirees."}
  ]'::jsonb),

('discord-audit-sync-worker', 'Worker Sync Audit Discord',
  'Synchronise periodiquement les audit logs Discord',
  '[
    {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true", "description": "Active ou desactive le worker."},
    {"key": "audit_sync_interval", "label": "Intervalle sync audit Discord (secondes)", "type": "number", "required": false, "default": "300", "description": "Frequence de polling des audit logs Discord via l API."}
  ]'::jsonb),

('export-worker', 'Worker Export',
  'Traite les demandes d export de donnees',
  '[
    {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true", "description": "Active ou desactive le worker."},
    {"key": "export_scan_interval", "label": "Intervalle scan jobs export (secondes)", "type": "number", "required": false, "default": "5", "description": "Frequence de depilage de la file export_jobs."}
  ]'::jsonb),

('temp-roles-worker', 'Worker Roles Temporaires',
  'Retire les roles temporaires expires',
  '[
    {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true", "description": "Active ou desactive le worker."},
    {"key": "temp_roles_scan_interval", "label": "Intervalle scan roles temporaires (secondes)", "type": "number", "required": false, "default": "60", "description": "Frequence de scan des roles temporaires a retirer."}
  ]'::jsonb)

ON CONFLICT (bot_name) DO NOTHING;

-- ══════════════════════════════════════════════════════════
-- 2. automod-bot : +2 cles (IA)
-- ══════════════════════════════════════════════════════════

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "text_enabled", "label": "Analyse IA texte activee", "type": "boolean", "required": false, "default": "true", "description": "Active l analyse IA des messages texte via le backend."},
  {"key": "vision_enabled", "label": "Analyse IA images activee", "type": "boolean", "required": false, "default": "true", "description": "Active l analyse IA des images sur les messages avec pieces jointes."}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key": "text_enabled"}]'::jsonb);

-- ══════════════════════════════════════════════════════════
-- 3. progression-bot : +1 cle
-- ══════════════════════════════════════════════════════════

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "xp_role_mode", "label": "Mode multiplicateur XP par role", "type": "text", "required": false, "default": "separate", "description": "Mode d application des multiplicateurs de role (separate, highest, stack)."}
]'::jsonb
WHERE bot_name = 'progression-bot'
  AND NOT (config_schema @> '[{"key": "xp_role_mode"}]'::jsonb);

-- ══════════════════════════════════════════════════════════
-- 4. coude-bot : +20 cles (dons alignes gift_*, prix shop, bugs)
--    + retrait casino_* (code mort) + retrait hp_regen_tick_secs (mauvais bot)
-- ══════════════════════════════════════════════════════════

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "combat_expire_secs", "label": "Expiration defi (secondes)", "type": "number", "required": false, "default": "86400", "description": "Duree avant qu un defi en attente expire (lu par le bot)."},
  {"key": "bet_delay_secs", "label": "Delai paris (secondes)", "type": "number", "required": false, "default": "300", "description": "Delai apres acceptation d un defi avant resolution (lu par le bot pour affichage)."},
  {"key": "gift_min_coins", "label": "Don minimum (coins)", "type": "number", "required": false, "default": "10", "description": "Montant minimum d un don de coins entre joueurs."},
  {"key": "gift_min_coins_after", "label": "Don minimum apres seuil (coins)", "type": "number", "required": false, "default": "50", "description": "Montant minimum applique apres un seuil d activite."},
  {"key": "gift_tax_percent", "label": "Taxe sur les dons (%)", "type": "number", "required": false, "default": "10", "description": "Pourcentage preleve sur chaque don de coins."},
  {"key": "gift_cooldown_secs", "label": "Cooldown dons (secondes)", "type": "number", "required": false, "default": "3600", "description": "Temps d attente entre deux dons."},
  {"key": "shop_potion_soin_price", "label": "Prix Potion de soin", "type": "number", "required": false, "default": "80", "description": "Prix de la Potion de soin au shop."},
  {"key": "shop_antidote_price", "label": "Prix Antidote", "type": "number", "required": false, "default": "150", "description": "Prix de l Antidote au shop."},
  {"key": "shop_potion_majeure_price", "label": "Prix Potion majeure", "type": "number", "required": false, "default": "200", "description": "Prix de la Potion majeure au shop."},
  {"key": "shop_bouclier_price", "label": "Prix Bouclier", "type": "number", "required": false, "default": "250", "description": "Prix du Bouclier au shop."},
  {"key": "shop_poison_price", "label": "Prix Poison", "type": "number", "required": false, "default": "300", "description": "Prix du Poison au shop."},
  {"key": "shop_masque_braquage_price", "label": "Prix Masque de braquage", "type": "number", "required": false, "default": "100", "description": "Prix du Masque (braquage)."},
  {"key": "shop_pied_de_biche_price", "label": "Prix Pied-de-biche", "type": "number", "required": false, "default": "150", "description": "Prix du Pied-de-biche (braquage)."},
  {"key": "shop_crochet_vault_price", "label": "Prix Crochet de coffre", "type": "number", "required": false, "default": "220", "description": "Prix du Crochet de coffre (braquage)."},
  {"key": "shop_plan_coffre_price", "label": "Prix Plan du coffre", "type": "number", "required": false, "default": "320", "description": "Prix du Plan du coffre (braquage)."},
  {"key": "shop_fumigene_diversion_price", "label": "Prix Fumigene (diversion)", "type": "number", "required": false, "default": "450", "description": "Prix du Fumigene (braquage)."},
  {"key": "shop_explosif_price", "label": "Prix Explosif", "type": "number", "required": false, "default": "600", "description": "Prix de l Explosif (braquage)."},
  {"key": "shop_hacker_kit_price", "label": "Prix Kit hacker", "type": "number", "required": false, "default": "800", "description": "Prix du Kit hacker (braquage)."},
  {"key": "shop_drone_espion_price", "label": "Prix Drone espion", "type": "number", "required": false, "default": "1000", "description": "Prix du Drone espion (braquage)."},
  {"key": "shop_equipe_de_pros_price", "label": "Prix Equipe de pros", "type": "number", "required": false, "default": "1500", "description": "Prix de l Equipe de pros (braquage)."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "shop_potion_soin_price"}]'::jsonb);

-- Retire casino_* (code mort apres refonte) et hp_regen_tick_secs (mauvais bot)
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM jsonb_array_elements(config_schema) AS elem
    WHERE elem->>'key' NOT IN (
      'casino_enabled', 'casino_max_bet', 'casino_cooldown_secs',
      'casino_max_daily', 'casino_max_daily_gain',
      'hp_regen_tick_secs'
    )
)
WHERE bot_name = 'coude-bot';

-- ══════════════════════════════════════════════════════════
-- 5. cache-worker : +3 cles
-- ══════════════════════════════════════════════════════════

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "leaderboards_refresh", "label": "Rafraichissement leaderboards (secondes)", "type": "number", "required": false, "default": "300", "description": "Frequence de regeneration du cache leaderboards."},
  {"key": "user_cache_sync", "label": "Sync cache utilisateurs (secondes)", "type": "number", "required": false, "default": "600", "description": "Frequence de synchronisation du cache users."},
  {"key": "partition_manager", "label": "Gestion partitions (secondes)", "type": "number", "required": false, "default": "3600", "description": "Frequence de maintenance des partitions PostgreSQL."}
]'::jsonb
WHERE bot_name = 'cache-worker'
  AND NOT (config_schema @> '[{"key": "leaderboards_refresh"}]'::jsonb);

-- ══════════════════════════════════════════════════════════
-- 6. coude-worker : +4 cles, retrait dead cles
-- ══════════════════════════════════════════════════════════

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "betting_check_secs", "label": "Intervalle resolution paris (secondes)", "type": "number", "required": false, "default": "30", "description": "Frequence de verification des combats en phase betting a resoudre."},
  {"key": "hp_regen_tick_secs", "label": "Frequence job regen HP (secondes)", "type": "number", "required": false, "default": "300", "description": "Frequence du tick de regeneration HP des joueurs."},
  {"key": "cashbox_tick_secs", "label": "Frequence check caisse communautaire (secondes)", "type": "number", "required": false, "default": "3600", "description": "Frequence de verification du declenchement de la redistribution de la caisse."},
  {"key": "cashbox_min_days", "label": "Jours min entre redistributions caisse", "type": "number", "required": false, "default": "7", "description": "Duree minimale entre deux redistributions de la caisse communautaire."}
]'::jsonb
WHERE bot_name = 'coude-worker'
  AND NOT (config_schema @> '[{"key": "betting_check_secs"}]'::jsonb);

-- Retire les cles jamais lues par le worker
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM jsonb_array_elements(config_schema) AS elem
    WHERE elem->>'key' NOT IN ('expiry_penalty_percent', 'refund_bets_on_expiry')
)
WHERE bot_name = 'coude-worker';

-- ══════════════════════════════════════════════════════════
-- 7. moderation-worker : +1 cle
-- ══════════════════════════════════════════════════════════

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "send_reminders_interval", "label": "Intervalle rappels sanctions (secondes)", "type": "number", "required": false, "default": "30", "description": "Frequence d envoi des rappels de sanctions aux moderateurs."}
]'::jsonb
WHERE bot_name = 'moderation-worker'
  AND NOT (config_schema @> '[{"key": "send_reminders_interval"}]'::jsonb);
