-- 003_community_life.sql
--
-- Les sections vivantes de l'espace membre : recherche de joueurs, sondages,
-- membre du mois, annonces du site.
--
-- Ces quatre concepts arrivent dans une seule migration parce qu'ils ne
-- servent qu'a une chose : remplir la page membre. Les separer aurait
-- fabrique quatre migrations qui n'ont de sens qu'ensemble.
--
-- Les anniversaires d'arrivee et les nouveaux membres n'ont PAS de table :
-- ils se deduisent de `guild_members.joined_at`. Dupliquer cette date
-- ailleurs, c'est garantir qu'elle divergera.


-- ─────────────────────────────────────────────────────────────────────
-- Cherche des joueurs
-- ─────────────────────────────────────────────────────────────────────
--
-- Une annonce est ephemere par nature : « je cherche 2 personnes pour ce
-- soir » n'a plus aucun sens demain. D'ou `expires_at`, obligatoire, plutot
-- qu'une suppression manuelle que personne ne fera jamais.
--
-- `slots` est le nombre de personnes RECHERCHEES, pas la taille du groupe :
-- c'est la formulation naturelle (« il me manque 2 gars ») et elle evite
-- d'avoir a demander combien on est deja.

CREATE TABLE IF NOT EXISTS community_lfg (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    guild_id character varying(20) NOT NULL,
    author_id character varying(20) NOT NULL,
    author_name text NOT NULL DEFAULT '',
    -- Jeu en texte libre : on cherche aussi des gens pour des jeux qu'on
    -- n'heberge pas.
    game character varying(80) NOT NULL,
    -- Rattachement facultatif a un serveur Nexus, pour afficher la jaquette
    -- et l'etat en ligne. Pas de cle etrangere : les serveurs vivent dans une
    -- autre base, un identifiant suffit.
    game_server_id uuid,
    slots integer NOT NULL,
    -- Quand on joue, en texte libre : « ce soir 21h », « le week-end »,
    -- « quand vous voulez ». Un timestamp aurait force a mentir sur les
    -- annonces sans horaire.
    when_text character varying(80) NOT NULL DEFAULT '',
    description text,
    -- Passe a false quand l'auteur a trouve son monde : l'annonce reste
    -- visible barree quelques heures plutot que de disparaitre d'un coup
    -- sous les yeux de ceux qui la lisaient.
    is_open boolean DEFAULT true NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,

    CONSTRAINT chk_community_lfg_slots CHECK (slots BETWEEN 1 AND 50)
);

-- Requete dominante : « les annonces ouvertes et non expirees de cette
-- guilde, les plus recentes d'abord ».
CREATE INDEX IF NOT EXISTS idx_community_lfg_live
    ON community_lfg USING btree (guild_id, created_at DESC)
    WHERE (is_open = true);

CREATE INDEX IF NOT EXISTS idx_community_lfg_expiry
    ON community_lfg USING btree (expires_at);

-- Table separee plutot qu'un compteur : on veut afficher QUI vient, et
-- pouvoir se desinscrire sans course a la mise a jour du compteur.
CREATE TABLE IF NOT EXISTS community_lfg_interest (
    lfg_id uuid NOT NULL REFERENCES community_lfg(id) ON DELETE CASCADE,
    user_id character varying(20) NOT NULL,
    username text NOT NULL DEFAULT '',
    joined_at timestamp with time zone DEFAULT now() NOT NULL,

    PRIMARY KEY (lfg_id, user_id)
);


-- ─────────────────────────────────────────────────────────────────────
-- Sondages
-- ─────────────────────────────────────────────────────────────────────
--
-- Trois tables plutot qu'un JSON d'options : on doit compter les voix par
-- option et empecher un membre de voter deux fois. La cle primaire
-- (poll_id, user_id) de community_poll_votes rend le double vote
-- structurellement impossible — pas seulement interdit par le code.

CREATE TABLE IF NOT EXISTS community_polls (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    guild_id character varying(20) NOT NULL,
    question character varying(200) NOT NULL,
    description text,
    -- Un sondage sans date de fin traine indefiniment sur la page.
    closes_at timestamp with time zone NOT NULL,
    -- Cloture manuelle anticipee.
    is_closed boolean DEFAULT false NOT NULL,
    is_public boolean DEFAULT true NOT NULL,
    created_by character varying(20) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,

    CONSTRAINT chk_community_polls_question CHECK (length(btrim(question)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_community_polls_live
    ON community_polls USING btree (guild_id, closes_at DESC);

CREATE TABLE IF NOT EXISTS community_poll_options (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    poll_id uuid NOT NULL REFERENCES community_polls(id) ON DELETE CASCADE,
    label character varying(120) NOT NULL,
    -- Couleur de la barre (hex sans #). Facultative : l'API retombe sur une
    -- palette par defaut.
    color character varying(8),
    -- Ordre d'affichage, fixe a la creation : sans lui, l'ordre des options
    -- changerait a chaque requete et le lecteur serait perdu.
    position integer NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_community_poll_options_poll
    ON community_poll_options USING btree (poll_id, position);

CREATE TABLE IF NOT EXISTS community_poll_votes (
    poll_id uuid NOT NULL REFERENCES community_polls(id) ON DELETE CASCADE,
    option_id uuid NOT NULL REFERENCES community_poll_options(id) ON DELETE CASCADE,
    user_id character varying(20) NOT NULL,
    voted_at timestamp with time zone DEFAULT now() NOT NULL,

    -- Un vote par personne et par sondage. Changer d'avis = UPSERT.
    PRIMARY KEY (poll_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_community_poll_votes_option
    ON community_poll_votes USING btree (option_id);


-- ─────────────────────────────────────────────────────────────────────
-- Membre du mois
-- ─────────────────────────────────────────────────────────────────────
--
-- Designation par le staff, et non calcul automatique sur l'activite : ce
-- qu'on veut recompenser (accueillir les nouveaux, relancer un vocal mort)
-- ne se mesure pas en nombre de messages. Un classement automatique
-- recompenserait le bavardage.
--
-- `reason` est NOT NULL et non vide pour cette raison exacte : sans le
-- pourquoi, la section n'est qu'un nom affiche.

CREATE TABLE IF NOT EXISTS community_spotlight (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL DEFAULT '',
    avatar text,
    -- Periode au format 'YYYY-MM'. Un seul membre du mois par mois.
    period character varying(7) NOT NULL,
    reason text NOT NULL,
    chosen_by character varying(20) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,

    CONSTRAINT uq_community_spotlight_period UNIQUE (guild_id, period),
    CONSTRAINT chk_community_spotlight_reason CHECK (length(btrim(reason)) > 0),
    CONSTRAINT chk_community_spotlight_period CHECK (period ~ '^[0-9]{4}-[0-9]{2}$')
);


-- ─────────────────────────────────────────────────────────────────────
-- Annonces du site
-- ─────────────────────────────────────────────────────────────────────
--
-- Distinct de la table `announcements` existante, qui pilote des messages
-- Discord recurrents postes par le bot (rappels de bump, etc.). Melanger les
-- deux ferait remonter « pensez a bump ! » dans les nouvelles du site.

CREATE TABLE IF NOT EXISTS community_news (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    guild_id character varying(20) NOT NULL,
    title character varying(160) NOT NULL,
    body text NOT NULL,
    -- Chemin RELATIF vers une image de web/public/imgs/, comme les jaquettes
    -- de jeu : stocker une URL absolue figerait le domaine en base.
    image_url text,
    -- Epinglee en tete de liste independamment de sa date.
    is_pinned boolean DEFAULT false NOT NULL,
    is_public boolean DEFAULT true NOT NULL,
    published_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by character varying(20) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,

    CONSTRAINT chk_community_news_title CHECK (length(btrim(title)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_community_news_recent
    ON community_news USING btree (guild_id, is_pinned DESC, published_at DESC);
