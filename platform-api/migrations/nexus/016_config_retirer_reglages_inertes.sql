-- 016_config_retirer_reglages_inertes.sql
--
-- Retire les reglages qui n'ont AUCUN service pour les lire.
--
-- Un curseur qu'on deplace sans effet est pire que son absence : il fait
-- croire au probleme resolu, et on cherche ailleurs quand le comportement ne
-- change pas. Mieux vaut une page plus courte et entierement fiable.
--
-- Ce qui part, et pourquoi :
--
--   xp_winner / xp_loser / xp_underdog_bonus / stat_points_per_level /
--   max_level / hp_regen_per_hour
--     La resolution de combat de Nexus n'attribue pas d'experience et ne
--     fait pas monter de niveau. Ces mecaniques existaient dans l'ancien
--     Coude (avant ff6e8a46) mais n'ont pas ete reprises. Les reglages
--     reviendront avec elles.
--
--   combat_cooldown_minutes / combat_mise_min / combat_mise_max
--     Les combats sont crees par le bot sans passer par un service qui
--     pourrait appliquer ces bornes.
--
--   transfer_fee_pct
--     Le transfert est atomique en base ; prelever des frais demande de
--     modifier cette transaction, pas seulement de lire un reglage. A faire
--     proprement, ou pas du tout.
--
--   bet_payout_multiplier
--     Le gain d'un pari est calcule a la resolution du combat, hors du
--     service de paris.
--
--   leaderboard_size
--     La taille du classement vient deja du client, qui demande ce qu'il
--     veut afficher.

DO $$
DECLARE
    inertes text[] := ARRAY[
        'xp_winner', 'xp_loser', 'xp_underdog_bonus', 'stat_points_per_level',
        'max_level', 'hp_regen_per_hour', 'combat_cooldown_minutes',
        'combat_mise_min', 'combat_mise_max', 'transfer_fee_pct',
        'bet_payout_multiplier', 'leaderboard_size'
    ];
    modele record;
    conserve jsonb;
    entree jsonb;
BEGIN
    FOR modele IN
        SELECT bot_name, config_schema FROM bot_definitions
        WHERE bot_name IN ('nexus-economy', 'nexus-coude')
    LOOP
        conserve := '[]'::jsonb;
        FOR entree IN SELECT * FROM jsonb_array_elements(modele.config_schema) LOOP
            IF NOT ((entree ->> 'key') = ANY (inertes)) THEN
                conserve := conserve || jsonb_build_array(entree);
            END IF;
        END LOOP;
        UPDATE bot_definitions SET config_schema = conserve
        WHERE bot_name = modele.bot_name;
    END LOOP;
END $$;

-- Les valeurs deja saisies pour ces cles n'ont plus de sens : les laisser
-- ferait resurgir un ancien reglage le jour ou la cle sera reutilisee.
DELETE FROM bot_guild_config
WHERE bot_name IN ('nexus-economy', 'nexus-coude')
  AND config_key IN (
    'xp_winner', 'xp_loser', 'xp_underdog_bonus', 'stat_points_per_level',
    'max_level', 'hp_regen_per_hour', 'combat_cooldown_minutes',
    'combat_mise_min', 'combat_mise_max', 'transfer_fee_pct',
    'bet_payout_multiplier', 'leaderboard_size'
  );

-- Le cout de l'assurance vaut 50 en base, pas 100 : le defaut affiche
-- annoncait un prix qui n'etait pas celui preleve.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem ->> 'key' = 'insurance_cost'
             THEN elem || '{"default": "50"}'::jsonb
             ELSE elem END
    )
    FROM jsonb_array_elements(config_schema) AS elem
)
WHERE bot_name = 'nexus-coude'
  AND config_schema @> '[{"key": "insurance_cost"}]'::jsonb;
