-- Ajout des 6 champs channel au schema de configuration du coude-bot.
-- ComponentConfigPage.vue les affichera automatiquement.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key":"channel_combats","label":"Salon combats & paris","type":"channel","required":false,"default":""},
  {"key":"channel_leaderboard","label":"Salon leaderboard","type":"channel","required":false,"default":""},
  {"key":"channel_profil","label":"Salon profil / shop / train","type":"channel","required":false,"default":""},
  {"key":"channel_activites","label":"Salon vol / casino / primes","type":"channel","required":false,"default":""},
  {"key":"channel_announcements","label":"Salon annonces (chaos quotidien)","type":"channel","required":false,"default":""},
  {"key":"channel_notifications","label":"Salon notifications combats","type":"channel","required":false,"default":""}
]'::jsonb
WHERE bot_name = 'coude';
