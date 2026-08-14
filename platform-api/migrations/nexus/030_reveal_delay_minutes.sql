-- 030_reveal_delay_minutes.sql
--
-- Ajoute le reglage « delai avant revelation de l'IP (minutes) » au schema du
-- module game-portal.
--
-- Contexte : le bouton « Reveler l'adresse IP » du panneau d'inscription ne
-- revele plus l'adresse immediatement. Il DEMARRE le serveur si besoin, annonce
-- l'ouverture dans le panneau, puis programme la revelation de l'IP dans le
-- salon prive au bout de ce delai. La valeur est propre a chaque guilde : elle
-- vit donc dans `bot_definitions.config_schema` / `bot_guild_config`, pas dans
-- une variable d'environnement (regle 6 du CLAUDE.md).
--
-- Defaut 10 minutes : laisse au conteneur le temps de finir son boot avant que
-- les joueurs recoivent l'adresse. Le worker reveal-ip (toutes les 5 min) ne
-- revele que les serveurs `running` dont l'echeance est passee, donc un delai
-- trop court n'ouvre jamais avant que le serveur soit reellement en ligne.
--
-- Idempotent : n'ajoute la cle que si elle est absente.

UPDATE bot_definitions
SET config_schema = config_schema || jsonb_build_array(
    jsonb_build_object(
        'key', 'reveal_delay_minutes',
        'type', 'number',
        'min', 1,
        'max', 1440,
        'unit', 'min',
        'label', 'Delai avant revelation de l IP (minutes)',
        'default', '10',
        'required', false,
        'description', 'Au clic sur « Reveler l''adresse IP », le serveur demarre et l''adresse est revelee dans le salon prive apres ce delai.'
    )
)
WHERE bot_name = 'game-portal'
  AND NOT (config_schema @> '[{"key": "reveal_delay_minutes"}]'::jsonb);
