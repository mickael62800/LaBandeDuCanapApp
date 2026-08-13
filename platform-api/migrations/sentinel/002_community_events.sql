-- 002_community_events.sql
--
-- Planning de la communaute : evenements et campagnes de jeu.
--
-- Modele volontairement fonde sur une PLAGE (starts_at -> ends_at) et non sur
-- un instant : une saison Minecraft ou une campagne Palworld tient plusieurs
-- semaines. Une soiree ponctuelle est simplement une plage de quelques heures.
-- C'est aussi ce qui ecarte les evenements planifies natifs de Discord, penses
-- pour une seance unique.
--
-- `all_day` distingue « le 12 fevrier » de « le 12 fevrier a 21h » : sans lui,
-- une campagne de trois semaines afficherait une heure de debut sans aucun sens.

CREATE TABLE IF NOT EXISTS community_events (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    guild_id character varying(20) NOT NULL,
    title character varying(120) NOT NULL,
    description text,
    -- Jeu concerne, en texte libre : le planning doit pouvoir annoncer un jeu
    -- qui n'a pas (ou pas encore) de serveur chez nous.
    game character varying(80),
    -- Couleur d'affichage dans le calendrier (hex sans #).
    color character varying(8),
    starts_at timestamp with time zone NOT NULL,
    ends_at timestamp with time zone NOT NULL,
    all_day boolean DEFAULT false NOT NULL,
    -- Visible par les visiteurs non connectes. Faux = reserve aux membres.
    is_public boolean DEFAULT true NOT NULL,
    -- Etat de publication : un evenement peut se preparer avant d'etre annonce.
    status character varying(16) DEFAULT 'published' NOT NULL,
    created_by character varying(20) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,

    CONSTRAINT chk_community_events_range CHECK (ends_at >= starts_at),
    CONSTRAINT chk_community_events_status
        CHECK (status IN ('draft', 'published', 'cancelled'))
);

-- Requete dominante : « les evenements de cette guilde qui chevauchent la
-- semaine/le mois affiche ». Un index sur la borne de debut suffit a la
-- restreindre, la borne de fin est ensuite filtree sur un petit ensemble.
CREATE INDEX IF NOT EXISTS idx_community_events_range
    ON community_events USING btree (guild_id, starts_at DESC);

CREATE INDEX IF NOT EXISTS idx_community_events_public
    ON community_events USING btree (guild_id, starts_at DESC)
    WHERE (is_public = true AND status = 'published');

-- Inscriptions. Table separee plutot qu'un tableau JSON : on veut compter,
-- lister et desinscrire sans reecrire tout l'evenement.
CREATE TABLE IF NOT EXISTS community_event_participants (
    event_id uuid NOT NULL REFERENCES community_events(id) ON DELETE CASCADE,
    user_id character varying(20) NOT NULL,
    username text NOT NULL DEFAULT '',
    -- 'going' | 'maybe' — un « peut-etre » est une information utile pour
    -- dimensionner une soiree.
    answer character varying(8) DEFAULT 'going' NOT NULL,
    registered_at timestamp with time zone DEFAULT now() NOT NULL,

    PRIMARY KEY (event_id, user_id),
    CONSTRAINT chk_community_event_answer CHECK (answer IN ('going', 'maybe'))
);
