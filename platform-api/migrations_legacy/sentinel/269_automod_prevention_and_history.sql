-- Automod — action "prevention" (cran sous le warn) + historique dans la carte.
--
-- 1) Autorise 'prevention' dans les CHECK des votes/decisions automod.
--    (suggested_action reste warn/delete/mute/ban : l'IA ne suggere pas
--     prevention, c'est un choix humain de vote.)
-- 2) Ajoute la cle de config card_history_count (nb d'antecedents affiches
--    dans la carte ; 0 = totaux seulement).

-- vote_action : ajoute 'prevention'
ALTER TABLE automod_review_votes DROP CONSTRAINT IF EXISTS automod_review_votes_vote_action_check;
ALTER TABLE automod_review_votes
    ADD CONSTRAINT automod_review_votes_vote_action_check
    CHECK (vote_action IN ('prevention','warn','delete','mute','ban','ignore'));

-- decided_action : ajoute 'prevention'
ALTER TABLE automod_reviews DROP CONSTRAINT IF EXISTS automod_reviews_decided_action_check;
ALTER TABLE automod_reviews
    ADD CONSTRAINT automod_reviews_decided_action_check
    CHECK (decided_action IS NULL OR decided_action IN ('prevention','warn','delete','mute','ban','ignore'));

-- applied_action : ajoute 'prevention'
ALTER TABLE automod_reviews DROP CONSTRAINT IF EXISTS automod_reviews_applied_action_check;
ALTER TABLE automod_reviews
    ADD CONSTRAINT automod_reviews_applied_action_check
    CHECK (applied_action IS NULL OR applied_action IN ('prevention','warn','delete','mute','ban','ignore'));

-- Cle de config (page web automod) : nb d'antecedents affiches dans la carte.
-- Idempotent : filtre puis re-agrege.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'card_history_count'
        UNION ALL SELECT '{
            "key": "card_history_count",
            "label": "Antecedents affiches dans la carte",
            "type": "number",
            "required": false,
            "default": "5",
            "description": "Nombre d''infractions passees (avec date) listees dans la carte de review/vote. 0 = afficher seulement les totaux (X warns, Y mutes, Z bans).",
            "min": 0,
            "max": 20
        }'::jsonb AS elem
    ) sub
)
WHERE bot_name = 'automod-bot';
