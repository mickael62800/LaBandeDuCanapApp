-- Restaure les cles que la migration 207 a oublie de reporter dans le
-- schema fusionne `coude-bot`. Le code Rust (bot + worker) lit toujours
-- ces cles depuis `bot_guild_config`, mais elles n'apparaissaient plus
-- dans la page Composants → impossible de les editer via l'UI.
--
-- Aucune valeur dans `bot_guild_config` n'est touchee : 207 n'avait
-- ecrase que `bot_definitions.config_schema`, pas les valeurs des
-- guilds. Les guilds qui avaient deja configure ces cles avant 207 les
-- retrouveront pre-remplies des que cette migration est appliquee.
--
-- Ajouts (8 cles) :
--   • Channels (origine mig 071/076/139) :
--       channel_combats, channel_leaderboard, channel_profil,
--       channel_activites, channel_announcements, channel_notifications,
--       tournament_channel_id
--   • Worker (origine mig 131) :
--       betting_check_secs, hp_regen_tick_secs, cashbox_tick_secs,
--       cashbox_min_days
--
-- Retraits (2 cles dead) :
--   • expiry_penalty_percent, refund_bets_on_expiry
--     (mig 131 les avait deja marquees "jamais lues" — 207 les a
--     reintroduites par erreur)

-- 1) Retire les 2 cles dead reintroduites par 207.
UPDATE bot_definitions
SET config_schema = COALESCE((
    SELECT jsonb_agg(elem)
    FROM jsonb_array_elements(config_schema::jsonb) elem
    WHERE elem->>'key' NOT IN ('expiry_penalty_percent', 'refund_bets_on_expiry')
), '[]'::jsonb)::jsonb
WHERE bot_name = 'coude-bot';

-- 2) Ajoute les 7 channel keys (idempotent : skip si deja presentes).
UPDATE bot_definitions
SET config_schema = config_schema::jsonb || '[
    {"key": "channel_combats", "label": "Salon combats & paris", "type": "channel", "required": false, "description": "Salon ou les commandes /coude, /coude_amical et /pari peuvent etre utilisees.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "channel_leaderboard", "label": "Salon leaderboard", "type": "channel", "required": false, "description": "Salon ou la commande /leaderboard peut etre utilisee.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "channel_profil", "label": "Salon profil / shop / train", "type": "channel", "required": false, "description": "Salon des commandes joueur : /profil, /shop, /train, /hp, /potion, /protection, /repos, /resume, /saison, /classe, /cagnotte, /reset_stats, /boost_voleur, /assurance.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "channel_activites", "label": "Salon activites (vol / casino / primes)", "type": "channel", "required": false, "description": "Salon des commandes d action : /voler, /braquage, /coalition, /contribuer_prime, /donner, /honneur, /maudire, /prank, /prestige, /prime, /saboter, /tout_ou_rien, /travaux, /vendetta.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "channel_announcements", "label": "Salon annonces", "type": "channel", "required": false, "description": "Salon pour les evenements automatiques : daily chaos, happy hour, bloodbath.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "channel_notifications", "label": "Salon notifications combats", "type": "channel", "required": false, "description": "Salon pour les alertes aux joueurs : nouveaux defis, paris ouverts.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "tournament_channel_id", "label": "Salon annonce tournoi", "type": "channel", "required": false, "description": "Salon ou poster le classement et le resultat hebdo du tournoi. Si vide, utilise le salon activites.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "channel_combats"}]'::jsonb);

-- 3) Ajoute les 4 worker keys (idempotent).
UPDATE bot_definitions
SET config_schema = config_schema::jsonb || '[
    {"key": "betting_check_secs", "label": "Worker : intervalle resolution paris", "type": "number", "required": false, "default": "30", "min": 1, "unit": "s", "description": "Frequence de verification des combats en phase betting a resoudre (sentinel-worker).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "hp_regen_tick_secs", "label": "Worker : frequence regen HP", "type": "number", "required": false, "default": "300", "min": 1, "unit": "s", "description": "Frequence du tick de regeneration HP des joueurs (sentinel-worker).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "cashbox_tick_secs", "label": "Worker : frequence check caisse communautaire", "type": "number", "required": false, "default": "3600", "min": 1, "unit": "s", "description": "Frequence de verification du declenchement de la redistribution de la caisse (sentinel-worker).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "cashbox_min_days", "label": "Jours min entre redistributions caisse", "type": "number", "required": false, "default": "7", "min": 1, "unit": "j", "description": "Duree minimale entre deux redistributions de la caisse communautaire.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "betting_check_secs"}]'::jsonb);
