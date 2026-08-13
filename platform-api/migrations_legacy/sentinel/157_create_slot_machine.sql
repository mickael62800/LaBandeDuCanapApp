-- Migration 157 : nouveau jeu "slot machine" / "tirette".
--
-- Mecanique : le joueur tire sur un panel persistant -> spin de 3 symboles
-- (RNG ponderee) -> 3 identiques = payout multiplie selon le symbole.
-- 1% de chaque mise alimente un jackpot progressif (debloque sur 3x 7).
-- Daily bonus : 1 spin gratuit / jour pour les joueurs ayant configure.

-- ══════════════════════════════════════════════════════════
-- Tables
-- ══════════════════════════════════════════════════════════

-- Pool jackpot progressif : un row par guild.
-- current_pool augmente de 1% de chaque mise. Quand jackpot decroche,
-- on credit le winner et on reset a starting_pool (cf. config).
CREATE TABLE IF NOT EXISTS slot_jackpot_pool (
    guild_id        VARCHAR(20) PRIMARY KEY,
    current_pool    BIGINT NOT NULL DEFAULT 0 CHECK (current_pool >= 0),
    last_won_by     VARCHAR(20),
    last_won_at     TIMESTAMPTZ,
    last_won_amount BIGINT,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Historique des spins. Chaque ligne = 1 spin.
-- symbols est un array JSONB de 3 strings (les emojis ou IDs).
-- payout = montant credit au joueur (0 si perdu, mise * mult si gagne).
CREATE TABLE IF NOT EXISTS slot_spin_log (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    VARCHAR(20) NOT NULL,
    user_id     VARCHAR(20) NOT NULL,
    username    VARCHAR(100) NOT NULL,
    mise        BIGINT NOT NULL CHECK (mise >= 0),
    symbols     JSONB NOT NULL,
    payout      BIGINT NOT NULL DEFAULT 0 CHECK (payout >= 0),
    multiplier  REAL NOT NULL DEFAULT 0,
    is_jackpot  BOOLEAN NOT NULL DEFAULT FALSE,
    is_free     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_slot_spin_log_guild_created
    ON slot_spin_log (guild_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_slot_spin_log_user_guild
    ON slot_spin_log (guild_id, user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_slot_spin_log_jackpot
    ON slot_spin_log (guild_id, created_at DESC) WHERE is_jackpot = TRUE;

-- Tracking du daily bonus : 1 row par (user, day).
-- L existence de la row pour CURRENT_DATE = daily deja claim.
CREATE TABLE IF NOT EXISTS slot_daily_claims (
    guild_id    VARCHAR(20) NOT NULL,
    user_id     VARCHAR(20) NOT NULL,
    day         DATE        NOT NULL,
    claimed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id, day)
);

-- ══════════════════════════════════════════════════════════
-- bot_definitions : entree slot-bot avec schema complet enrichi
-- (descriptions, units, min/max, enum) — comme les bots recents.
-- ══════════════════════════════════════════════════════════
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'slot-bot',
    'Slot Machine',
    'Jeu de tirette type machine a sous : 3 symboles aleatoires, 3 identiques = payout multiplie. Jackpot progressif sur 3x 7.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true",
         "description": "Active ou desactive le module slot. Si OFF, le panel ne repond plus aux clics."},

        {"key": "panel_channel_id", "label": "Salon du panel", "type": "channel", "required": false,
         "description": "Salon ou est poste le panel persistant avec le bouton Tirer. Configure via /slot-setup."},

        {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false,
         "description": "Salon ou sont logges les jackpots et grosses victoires. Vide = pas de log."},

        {"key": "min_bet", "label": "Mise min", "type": "number", "required": false, "default": "10",
         "unit": "coins", "min": 1, "max": 1000000,
         "description": "Mise minimale par spin."},

        {"key": "max_bet", "label": "Mise max", "type": "number", "required": false, "default": "1000",
         "unit": "coins", "min": 1, "max": 100000000,
         "description": "Mise maximale par spin."},

        {"key": "default_bet", "label": "Mise par defaut", "type": "number", "required": false, "default": "50",
         "unit": "coins", "min": 1, "max": 1000000,
         "description": "Mise pre-selectionnee dans le panel quand le joueur clique Tirer."},

        {"key": "cooldown_secs", "label": "Cooldown entre spins", "type": "number", "required": false, "default": "5",
         "unit": "secondes", "min": 0, "max": 3600,
         "description": "Delai entre 2 spins pour un meme joueur. Anti-spam."},

        {"key": "symbols", "label": "Symboles (CSV)", "type": "text", "required": false,
         "default": "🍒,🍋,🍊,🍇,🔔,⭐,7️⃣",
         "description": "Liste des symboles separes par virgules. Du plus frequent au plus rare. Le dernier = jackpot."},

        {"key": "weights", "label": "Poids des symboles (CSV)", "type": "text", "required": false,
         "default": "30,25,20,15,7,2,1",
         "description": "Poids RNG de chaque symbole (meme ordre que symbols). Plus le poids est grand, plus le symbole sort souvent. Total libre."},

        {"key": "payout_3x_multipliers", "label": "Multiplicateurs 3 identiques (CSV)", "type": "text", "required": false,
         "default": "2,3,5,8,12,25,100",
         "description": "Multiplicateur de la mise pour 3 identiques (meme ordre que symbols). Le dernier = jackpot, declenche le pot progressif."},

        {"key": "payout_2x_enabled", "label": "Payout sur 2 identiques", "type": "boolean", "required": false, "default": "true",
         "description": "Si ON, 2 symboles identiques sur 3 = remboursement de la mise (1x). Reduit la frustration."},

        {"key": "jackpot_pool_share_pct", "label": "% mise vers jackpot", "type": "number", "required": false, "default": "1",
         "unit": "%", "min": 0, "max": 50,
         "description": "Pourcentage de chaque mise qui alimente le pool jackpot progressif. Recommande : 1-5%."},

        {"key": "jackpot_starting_pool", "label": "Pool jackpot de depart", "type": "number", "required": false, "default": "1000",
         "unit": "coins", "min": 0, "max": 100000000,
         "description": "Valeur de depart du pool jackpot (et reset a chaque jackpot decroche)."},

        {"key": "daily_bonus_enabled", "label": "Daily bonus actif", "type": "boolean", "required": false, "default": "true",
         "description": "Si ON, chaque joueur peut faire 1 spin gratuit par jour."},

        {"key": "daily_bonus_mise", "label": "Mise du spin gratuit", "type": "number", "required": false, "default": "100",
         "unit": "coins", "min": 1, "max": 1000000,
         "description": "Mise utilisee pour le spin gratuit quotidien (le payout suit ce mise)."},

        {"key": "panel_message", "label": "Message du panel", "type": "text", "required": false,
         "default": "🎰 **Machine a sous** 🎰\n\nClique sur **Tirer** pour faire un spin !\n3 identiques = jackpot.",
         "description": "Texte affiche dans le panel persistant. Markdown supporte."}
    ]'::jsonb
)
ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
