-- Tamagotchi — nouveau jeu de compagnon virtuel (module tamagotchi-bot).
--
-- Jeu INDEPENDANT de Coup de Coude : stats/combat/ELO/competences propres.
-- Seuls les COINS sont partages (wallet commun, table wallets existante).
--
-- M1 (fondation) : un compagnon par joueur, jauges de soin (faim/bonheur/
-- energie), stats de combat, statut sante (healthy/sick/dead), niveau/XP.
-- Le combat dedie, les competences, la boutique et la tenue viendront en
-- jalons ulterieurs.

CREATE TABLE IF NOT EXISTS pets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    name TEXT NOT NULL,
    -- Espece parmi ~6 (sanglier, ...). Stockee en texte, valeurs gerees cote
    -- domaine (enum Species).
    species TEXT NOT NULL,
    -- Specialisation/trait optionnel (ex: "entrainement_pp").
    specialization TEXT,

    level INT NOT NULL DEFAULT 1,
    xp BIGINT NOT NULL DEFAULT 0,
    born_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Jauges de soin (0-100).
    hunger INT NOT NULL DEFAULT 100,
    happiness INT NOT NULL DEFAULT 100,
    energy INT NOT NULL DEFAULT 100,

    -- Etat de sante : healthy | sick | dead.
    status TEXT NOT NULL DEFAULT 'healthy'
        CHECK (status IN ('healthy','sick','dead')),
    -- Depuis quand la faim est a 0 (pour le delai avant maladie).
    hunger_zero_since TIMESTAMPTZ,
    -- Depuis quand le compagnon est malade (pour le delai avant mort).
    sick_since TIMESTAMPTZ,
    died_at TIMESTAMPTZ,

    -- Stats de combat (propres a ce jeu).
    str INT NOT NULL DEFAULT 10,
    vit INT NOT NULL DEFAULT 10,
    agi INT NOT NULL DEFAULT 10,
    stat_points INT NOT NULL DEFAULT 0,

    -- Classement combat.
    elo INT NOT NULL DEFAULT 1000,
    wins INT NOT NULL DEFAULT 0,
    losses INT NOT NULL DEFAULT 0,

    -- Cooldowns par action : { "feed": "<rfc3339>", "sleep": ... }.
    cooldowns JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- Dernier tick de decroissance applique (idempotence du worker).
    last_decay_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Un seul compagnon vivant par joueur et par serveur.
    UNIQUE (guild_id, owner_id)
);

CREATE INDEX IF NOT EXISTS idx_pets_guild ON pets (guild_id);
-- Pour le tick worker : compagnons vivants a faire decroitre.
CREATE INDEX IF NOT EXISTS idx_pets_alive_decay
    ON pets (last_decay_at)
    WHERE status <> 'dead';

-- Journal des actions (carte "Dernieres actions").
CREATE TABLE IF NOT EXISTS pet_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pet_id UUID NOT NULL REFERENCES pets(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    detail TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_pet_events_pet ON pet_events (pet_id, created_at DESC);

-- Definition du bot pour la page Composants (config dans migration 256).
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'tamagotchi-bot',
    'Tamagotchi',
    'Compagnon virtuel : nourris, joue, soigne et fais evoluer ton animal.',
    '[{"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active le jeu Tamagotchi."}]'::jsonb
)
ON CONFLICT (bot_name) DO UPDATE
    SET display_name = EXCLUDED.display_name,
        description = EXCLUDED.description;
