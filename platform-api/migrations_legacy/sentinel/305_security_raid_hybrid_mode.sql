-- Mode hybride "suggest vs auto" pour la reponse anti-raid du security-bot.
--
-- La reponse GUILD-WIDE (lockdown + slowmode + bump de verification) peut
-- desormais etre appliquee automatiquement, seulement suggeree au staff, ou
-- (mode hybride) appliquee auto quand le raid est massif et suggeree sinon.
-- La quarantaine + le captcha (par compte, faible rayon d'action) restent
-- toujours automatiques.
--
-- Idempotent (miroir de 304) : on retire toute entree existante avec ces cles
-- avant de les re-ajouter, donc rejouer la migration ne cree pas de doublon.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' NOT IN (
                'raid_mode',
                'raid_auto_threshold',
                'raid_suggest_channel_id'
            )
        UNION ALL SELECT '{
            "key": "raid_mode",
            "type": "enum",
            "required": false,
            "default": "hybrid",
            "options": ["auto", "suggest", "hybrid"],
            "label": "Mode de réponse anti-raid",
            "description": "Mode de réponse anti-raid : auto (applique directement), suggest (demande confirmation staff), hybrid (auto si raid massif, sinon suggestion)."
        }'::jsonb
        UNION ALL SELECT '{
            "key": "raid_auto_threshold",
            "type": "number",
            "required": false,
            "default": "85",
            "min": 0,
            "max": 100,
            "label": "Anti-raid — seuil auto",
            "description": "En mode hybride : score de raid à partir duquel la réponse (lockdown/slowmode) est appliquée automatiquement ; en dessous elle est seulement suggérée au staff."
        }'::jsonb
        UNION ALL SELECT '{
            "key": "raid_suggest_channel_id",
            "type": "channel",
            "required": false,
            "default": "",
            "label": "Salon d''alerte anti-raid",
            "description": "Salon d''alerte anti-raid (suggestions). Vide -> repli sur le salon de logs sécurité, sinon application auto (protection avant silence)."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'security-bot';
