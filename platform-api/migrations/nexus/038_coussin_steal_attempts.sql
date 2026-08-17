-- 038_coussin_steal_attempts.sql
--
-- Fenetre de defense sur la fouille sous les coussins.
--
-- Le vol se jouait a pile ou face : un pourcentage fixe (30 %), sans que la
-- cible puisse quoi que ce soit. Perdre sept fois sur dix sans avoir eu son
-- mot a dire n'est pas un jeu, c'est une taxe — et c'est exactement ce que
-- les joueurs ont ressenti.
--
-- On reprend le modele de l'ancien Coup de Coude : la tentative reste OUVERTE
-- le temps que la victime reagisse. Si elle serre les coussins a temps, elle
-- garde toute sa defense ; sinon elle encaisse un malus de vigilance et le
-- voleur passe beaucoup plus facilement.
--
-- La tentative doit etre persistee, et pas seulement vivre dans le message
-- Discord : le bot peut redemarrer pendant la fenetre, et une tentative
-- oubliee laisserait la victime croire qu'elle s'en est tiree.

CREATE TABLE IF NOT EXISTS nexus_coussin_steal_attempts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    VARCHAR(20) NOT NULL,
    thief_id    VARCHAR(20) NOT NULL,
    victim_id   VARCHAR(20) NOT NULL,
    -- Ou le bot devra publier le denouement, y compris apres un redemarrage.
    channel_id  VARCHAR(20) NOT NULL,
    message_id  VARCHAR(20),
    status      VARCHAR(16) NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending', 'resolved')),
    -- Fin de la fenetre de defense. Passe cette date, l'absence de reaction
    -- vaut reponse : le job resout avec le malus.
    expires_at  TIMESTAMPTZ NOT NULL,
    -- Reglee au moment de la resolution : la victime a-t-elle reagi a temps ?
    defended    BOOLEAN,
    success     BOOLEAN,
    amount      BIGINT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

-- Le job ne cherche que les tentatives echues : l'index porte exactement sur
-- ce predicat, et sur rien d'autre.
CREATE INDEX IF NOT EXISTS idx_coussin_steal_pending
    ON nexus_coussin_steal_attempts (expires_at)
    WHERE status = 'pending';

-- Une seule fouille en cours par couple voleur/victime : sans cela, enchainer
-- les commandes ouvrirait dix fenetres simultanees sur la meme personne.
CREATE UNIQUE INDEX IF NOT EXISTS idx_coussin_steal_one_pending
    ON nexus_coussin_steal_attempts (guild_id, thief_id, victim_id)
    WHERE status = 'pending';

COMMENT ON COLUMN nexus_coussin_steal_attempts.defended IS
    'NULL tant que la tentative est ouverte. Ensuite : true = la victime a reagi dans la fenetre, false = elle a laisse passer.';
