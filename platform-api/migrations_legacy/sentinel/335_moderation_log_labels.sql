-- Moderation-bot — clarifie les libellés des DEUX salons de log de sanction,
-- pour qu'on comprenne quelle carte va dans quel salon (ils sont independants) :
--   * log_channel_id           -> grosse carte DETAILLEE (Strike, gravite...).
--   * sanctions_log_channel_id -> carte RECAP 2 lignes (qui/quoi/raison), ideale
--                                 pour un salon "commandes".
-- Idempotent : on retire les deux cles puis on les reinsere.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' NOT IN ('log_channel_id', 'sanctions_log_channel_id')
        UNION ALL SELECT '{
            "key": "log_channel_id",
            "label": "Salon des logs détaillés (carte complète)",
            "type": "channel",
            "required": false,
            "description": "Salon de la carte détaillée d une sanction (cible, modérateur, gravité, strikes, raison). Indépendant du salon récap.",
            "depends_on": {"key": "enabled", "equals": "true"}
        }'::jsonb
        UNION ALL SELECT '{
            "key": "sanctions_log_channel_id",
            "label": "Salon du récap des sanctions (carte 2 lignes)",
            "type": "channel",
            "required": false,
            "description": "Salon de la carte récap 2 lignes (qui a sanctionné qui + raison). À mettre dans un salon différent, par ex. un salon commandes. Vide = désactivé.",
            "depends_on": {"key": "enabled", "equals": "true"}
        }'::jsonb
    ) sub
)
WHERE bot_name = 'moderation-bot';
