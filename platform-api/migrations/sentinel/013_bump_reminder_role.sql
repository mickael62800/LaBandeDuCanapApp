-- Bump : role a mentionner (pinger) dans le message de rappel de fin de
-- cooldown. Optionnel (vide = pas de ping). Ajoute a la config bump-bot.

UPDATE bot_definitions SET
    config_schema = config_schema || jsonb_build_array(
        jsonb_build_object(
            'key', 'bump_reminder_role_id',
            'type', 'role',
            'label', 'Role a pinger au rappel',
            'required', false,
            'depends_on', jsonb_build_object('key', 'bump_reminder_enabled', 'equals', 'true'),
            'description', 'Role mentionne dans le rappel de fin de cooldown (quand le serveur peut etre re-bumpe). Vide = aucun ping.'
        )
    )
WHERE bot_name = 'bump-bot'
  AND NOT (config_schema @> '[{"key": "bump_reminder_role_id"}]'::jsonb);
