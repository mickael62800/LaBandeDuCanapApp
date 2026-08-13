-- Migration 171 : Catalogue de templates "flavor" Coup de Coude
-- (Phase 3 #9 audit). Centralise les templates de raillerie / vol /
-- braquage / prank dans une table editable runtime, plutot que des
-- arrays Rust hardcodees redéployables.
--
-- Cles attendues (voir 172_seed_*) :
--  - steal_success_afk     (vol reussi sur cible AFK)
--  - steal_success_fight   (vol reussi sur cible defendue)
--  - steal_fail            (vol echoue / contre-attaque)
--  - heist_success         (braquage reussi)
--  - heist_fail            (braquage echoue, prison)
--  - prank_scoop           (prank "faux scoop")
--  - prank_appel           (prank "faux appel DM")
--
-- Le champ `weight` permet de pondérer la sélection : valeur >= 1, default 1
-- (uniforme). Locale par défaut "fr".

CREATE TABLE IF NOT EXISTS coude_flavor_templates (
    id         UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    key        VARCHAR(64)  NOT NULL,
    locale     VARCHAR(8)   NOT NULL DEFAULT 'fr',
    weight     INTEGER      NOT NULL DEFAULT 1 CHECK (weight >= 1),
    content    TEXT         NOT NULL CHECK (length(content) > 0),
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Lookup principal : "tirer un template aleatoire pour (key, locale)".
-- Utilise par RANDOM() ORDER BY ou TABLESAMPLE selon la volumetrie.
CREATE INDEX IF NOT EXISTS idx_coude_flavor_templates_key_locale
    ON coude_flavor_templates (key, locale);
