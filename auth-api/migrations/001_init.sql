-- Base de l'identite : sessions web OAuth2 et journal des logins reussis.
--
-- POURQUOI UNE BASE A PART
--
-- Ces deux tables vivaient dans `discord_sentinel`. Consequence : toute
-- plateforme voulant savoir QUI appelle devait passer par sentinel-api, qui
-- devenait une dependance d'execution de Nexus, d'Atrium et de l'exploitation
-- — celle qui, si elle tombe, ferme le back-office entier. Le meme geste a
-- deja ete fait pour l'exploitation (ops-core / ops-api) : l'identite
-- n'appartient pas plus a Sentinel que les sondes de la machine hote.
--
-- La reprise des donnees existantes N'EST PAS faite ici : Postgres ne sait pas
-- requeter entre bases logiques. Elle est le fait du one-shot
-- `auth-data-import` du compose, qui tourne apres cette migration et copie les
-- lignes de discord_sentinel avec `ON CONFLICT DO NOTHING`. Cette migration-ci
-- doit pouvoir s'appliquer sur une installation neuve, sans base Sentinel en
-- face.

CREATE TABLE IF NOT EXISTS web_oauth_sessions (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    discord_user_id text NOT NULL,
    username text DEFAULT ''::text NOT NULL,
    global_name text,
    avatar text,
    -- Jetons Discord. Ils vivent ici et nulle part ailleurs : c'est cette
    -- table qui justifie a elle seule que la base soit separee et son role
    -- restreint.
    access_token text NOT NULL,
    refresh_token text NOT NULL,
    access_expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used_at timestamp with time zone DEFAULT now() NOT NULL
);

-- Le menage des sessions dormantes se fait par `last_used_at` : sans index,
-- il devient un scan complet a mesure que les sessions s'accumulent.
CREATE INDEX IF NOT EXISTS idx_web_oauth_sessions_last_used
    ON web_oauth_sessions USING btree (last_used_at);

CREATE TABLE IF NOT EXISTS successful_logins (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    discord_user_id text NOT NULL,
    username text DEFAULT ''::text NOT NULL,
    client_ip text DEFAULT ''::text NOT NULL,
    user_agent text DEFAULT ''::text NOT NULL,
    logged_at timestamp with time zone DEFAULT now() NOT NULL
);

-- L'ecran de securite lit les N derniers et purge par anciennete : les deux
-- passent par `logged_at`.
CREATE INDEX IF NOT EXISTS idx_successful_logins_logged_at
    ON successful_logins USING btree (logged_at DESC);
