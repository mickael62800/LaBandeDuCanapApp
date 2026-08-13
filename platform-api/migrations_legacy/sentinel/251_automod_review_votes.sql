-- Automod — refonte en systeme de VOTE des moderateurs.
--
-- Avant : une detection automod postait une carte de review, et UN seul
-- moderateur cliquait pour appliquer une action immediatement.
--
-- Apres : la detection ouvre un VOTE. Les moderateurs votent une sanction
-- (warn/delete/mute/ban/ignore). A l'echeance (delai configurable), on
-- compte les votes exprimes (majorite + quorum minimum). Le verdict passe
-- en 'decided', puis un ADMINISTRATEUR finalise via un bouton dedie (meme
-- pour un refus). Tout est parametrable cote web (cf. config_schema).
--
-- Cycle de vie du statut automod_reviews :
--   voting    -> vote ouvert (deadline en cours)
--   decided   -> deadline passee, verdict calcule, en attente de l'admin
--   applied   -> admin a applique la sanction
--   ignored   -> clos sans sanction (verdict ignore / quorum non atteint / refus)
--
-- ('pending' historique est conserve dans le CHECK pour les lignes
-- anterieures a la refonte ; les nouvelles cartes naissent en 'voting'.)

-- 1. Etendre automod_reviews.
ALTER TABLE automod_reviews
    ADD COLUMN IF NOT EXISTS voting_deadline TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS decided_action TEXT
        CHECK (decided_action IS NULL OR decided_action IN ('warn','delete','mute','ban','ignore')),
    ADD COLUMN IF NOT EXISTS quorum_met BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS decided_at TIMESTAMPTZ;

-- 2. Elargir le CHECK status pour inclure les nouveaux etats.
ALTER TABLE automod_reviews DROP CONSTRAINT IF EXISTS automod_reviews_status_check;
ALTER TABLE automod_reviews
    ADD CONSTRAINT automod_reviews_status_check
    CHECK (status IN ('pending','voting','decided','applied','ignored'));

-- 3. Index pour le job worker de cloture (votes a echeance).
CREATE INDEX IF NOT EXISTS idx_automod_reviews_voting_deadline
    ON automod_reviews (voting_deadline)
    WHERE status = 'voting';

-- 4. Table des votes : un vote par (review, moderateur), upsert pour changer.
CREATE TABLE IF NOT EXISTS automod_review_votes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    review_id UUID NOT NULL REFERENCES automod_reviews(id) ON DELETE CASCADE,
    voter_id TEXT NOT NULL,
    voter_name TEXT NOT NULL DEFAULT '',
    vote_action TEXT NOT NULL CHECK (vote_action IN ('warn','delete','mute','ban','ignore')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (review_id, voter_id)
);

CREATE INDEX IF NOT EXISTS idx_automod_review_votes_review
    ON automod_review_votes (review_id);
