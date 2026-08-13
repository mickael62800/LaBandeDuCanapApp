-- 022_wheel_cases.sql
--
-- Les cases de la Roue deviennent des donnees de serveur.
--
-- Elles etaient dix constantes dans le code : regler la rarete de la licorne
-- ou ajouter une case demandait de recompiler, ce qui revient a ne jamais le
-- faire. Le seul levier existant — un multiplicateur global — deplaçait tous
-- les gains ensemble, sans jamais changer la forme de la roue.
--
-- ABSENCE = ROUE HISTORIQUE. Aucune ligne n'est semee ici : une guilde sans
-- ligne joue les dix cases d'origine. C'est ce qui permet de livrer sans
-- toucher aux serveurs existants, et ce qui rend « revenir a la roue de
-- base » aussi simple que tout supprimer.

CREATE TABLE IF NOT EXISTS nexus_wheel_cases (
    guild_id VARCHAR(20) NOT NULL,
    -- Identifiant stable de la case. Il voyage jusqu'au site, qui s'en sert
    -- pour retrouver le secteur a mettre en avant apres un tirage.
    key VARCHAR(32) NOT NULL,
    label VARCHAR(120) NOT NULL,
    -- Negatif = perte, 0 = case blanche. Borne large : c'est au reglage de
    -- decider ce qui est raisonnable, pas au schema.
    payout BIGINT NOT NULL,
    -- Poids de tirage relatif. Au moins 1 : une case de poids nul ne sortirait
    -- jamais et occuperait l'ecran pour rien.
    weight INT NOT NULL CHECK (weight >= 1),
    -- Ordre d'affichage sur la roue dessinee.
    position INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, key)
);

CREATE INDEX IF NOT EXISTS idx_nexus_wheel_cases_guild
    ON nexus_wheel_cases (guild_id, position);
