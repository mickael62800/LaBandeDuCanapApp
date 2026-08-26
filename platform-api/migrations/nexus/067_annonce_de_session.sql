-- 067_annonce_de_session.sql
--
-- L'ouverture d'une session commence desormais par une annonce redigee par
-- Atrium, publiee AVANT le panneau d'inscription.
--
-- POURQUOI UNE COLONNE PLUTOT QU'UN SIMPLE ESSAI. L'annonce est un prealable :
-- quand Atrium ne peut pas ecrire, rien n'est publie et la reprise retente.
-- Sans trace en base, cette reprise ne saurait ni quelles sessions attendent
-- encore, ni lesquelles ont deja recu leur annonce — et republierait la meme
-- annonce a chaque passage.
--
-- `announcement_posted_at` est pose des que l'annonce EST PUBLIEE, avant meme
-- le panneau. Un panneau rate se rejoue sans dommage ; une annonce publiee
-- deux fois se voit.
--
-- `announcement_attempts` borne la reprise. Une panne prolongee de l'IA ne doit
-- pas faire retenter une session indefiniment : au-dela du plafond, on cesse et
-- l'exploitant est prevenu, plutot que d'accumuler des appels qui echouent tous
-- de la meme facon.

ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS announcement_posted_at timestamptz,
    ADD COLUMN IF NOT EXISTS announcement_attempts  integer NOT NULL DEFAULT 0;

COMMENT ON COLUMN game_servers.announcement_posted_at IS
    'Instant ou l''annonce Atrium a ete publiee. NULL = pas encore publiee, la reprise repassera.';
COMMENT ON COLUMN game_servers.announcement_attempts IS
    'Tentatives de redaction deja faites. Borne la reprise : une panne prolongee ne doit pas retenter sans fin.';

-- Les sessions deja ouvertes AVANT cette migration ont leur panneau publie et
-- n'attendent aucune annonce. Sans cela, la reprise les prendrait toutes pour
-- des sessions en souffrance et publierait une annonce sous chacune, des
-- semaines apres leur ouverture.
UPDATE game_servers
SET announcement_posted_at = COALESCE(started_at, created_at)
WHERE text_channel_id IS NOT NULL
  AND announcement_posted_at IS NULL;
