-- Journal d'événements par jeu : ajoute une clé `<jeu>_log_channel_id` (type
-- channel, vide par défaut = aucun log) au schéma de config de chaque jeu.
-- Append jsonb, idempotent.
UPDATE bot_definitions b
SET config_schema = b.config_schema
    || jsonb_build_array(jsonb_build_object(
        'key', v.cfg_key,
        'label', 'Salon de logs',
        'type', 'channel',
        'required', false,
        'default', '',
        'description', 'Salon où le bot journalise les événements du jeu (parties, gains, actions). Vide = aucun log.'
    ))
FROM (VALUES
    ('coude-bot', 'coude_log_channel_id'),
    ('influence-bot', 'influence_log_channel_id'),
    ('blackjack-bot', 'blackjack_log_channel_id'),
    ('slot-bot', 'slot_log_channel_id'),
    ('wheel-bot', 'wheel_log_channel_id'),
    ('tamagotchi-bot', 'tamagotchi_log_channel_id')
) AS v(bot, cfg_key)
WHERE b.bot_name = v.bot
  AND NOT (b.config_schema @> jsonb_build_array(jsonb_build_object('key', v.cfg_key)));
