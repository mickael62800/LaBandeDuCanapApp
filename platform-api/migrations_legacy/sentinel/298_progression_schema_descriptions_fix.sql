-- Corrige des descriptions trompeuses du schema de config progression-bot.
--
-- 1. level_up_channel_id / levelup_announce_enabled : les "role rewards" ont
--    ete supprimes (migration 250_drop_level_rewards). Le schema pretendait a
--    tort "les role rewards restent appliques".
-- 2. levelup_dm_enabled / levelup_message : marques "TODO pas encore cable"
--    alors qu'ils SONT cables dans announce_level_up (DM + template).
--
-- REPLACE chirurgical sur le JSONB (idempotent : no-op si la chaine est absente
-- ou deja corrigee).

UPDATE bot_definitions SET config_schema = REPLACE(
    REPLACE(
        REPLACE(
            config_schema::text,
            'Si OFF, aucun message dans le salon (les role rewards restent appliques).',
            'Si OFF, aucun message de level-up n est poste dans le salon.'
        ),
        'TODO : pas encore cable (envoi DM au membre lors du level-up).',
        'Envoie un DM prive au membre lors de son level-up.'
    ),
    'TODO : template pas encore cable. Variables prevues : {user}, {level}.',
    'Template du message de level-up. Variables : {user}, {level}, {kind}.'
)::jsonb
WHERE bot_name = 'progression-bot';
