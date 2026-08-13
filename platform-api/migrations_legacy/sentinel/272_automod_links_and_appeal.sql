-- Automod — suppression auto des liens non autorises + notification DSA.
--
--   auto_delete_links_enabled    : supprime automatiquement les liens non
--                                  autorises HORS image (lien etrange/non permis),
--                                  meme en moderation 100% humaine. Tracabilite
--                                  via le salon de logs + la detection persistee.
--   auto_protect_notify_member   : informe le membre en DM (motif + droit d'appel
--                                  via /appeal) quand une protection auto est
--                                  appliquee (conformite DSA).
--
-- Idempotent : on retire d'abord les cles si presentes, puis on les (re)ajoute.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' NOT IN ('auto_delete_links_enabled', 'auto_protect_notify_member')
        UNION ALL SELECT '{
            "key": "auto_delete_links_enabled",
            "label": "Supprimer automatiquement les liens non autorises (hors image)",
            "type": "boolean",
            "required": false,
            "default": "true",
            "description": "Si ON, un lien detecte comme non autorise/etrange (hors image) est supprime immediatement par le bot, meme en moderation 100% humaine. Tracabilite via le salon de logs et la detection enregistree."
        }'::jsonb
        UNION ALL SELECT '{
            "key": "auto_protect_notify_member",
            "label": "Informer le membre en DM (motif + droit d''appel)",
            "type": "boolean",
            "required": false,
            "default": "true",
            "description": "Si ON, lorsqu''une protection automatique (mute) est appliquee, le membre recoit un message prive avec le motif et la possibilite de contester via /appeal (conformite DSA)."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'automod-bot';
