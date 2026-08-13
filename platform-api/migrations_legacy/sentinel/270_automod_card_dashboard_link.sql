-- Automod — carte simplifiee : on retire le detail verbeux de la carte Discord
-- (incidents listes, contexte, antecedents dates) au profit d'un LIEN vers le
-- dashboard web qui affiche tout le detail.
--
-- 1) Retire card_history_count (la carte n'affiche plus que les totaux).
-- 2) Ajoute dashboard_base_url : URL de base du dashboard (par serveur) pour
--    construire le lien "Voir le detail". Vide = pas de bouton lien.
-- Idempotent : filtre puis re-agrege.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' NOT IN ('card_history_count', 'dashboard_base_url')
        UNION ALL SELECT '{
            "key": "dashboard_base_url",
            "label": "URL du dashboard (lien depuis les cartes)",
            "type": "text",
            "required": false,
            "description": "Base URL du dashboard web (ex: https://dash.exemple.com). Sert a generer le bouton \"Voir le detail\" sur les cartes de review/vote. Vide = pas de bouton."
        }'::jsonb AS elem
    ) sub
)
WHERE bot_name = 'automod-bot';

-- Purge des valeurs eventuellement enregistrees pour la cle retiree.
DELETE FROM bot_guild_config
WHERE bot_name = 'automod-bot' AND config_key = 'card_history_count';
