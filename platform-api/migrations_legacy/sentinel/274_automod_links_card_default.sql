-- Automod — politique liens revisée (CR revue modération).
--
-- Par défaut, un lien générique non autorisé HORS image part désormais en CARTE
-- (décision humaine), plus en suppression sèche. La suppression automatique
-- reste possible mais devient un opt-in explicite (mode agressif).
-- Phishing / invitation Discord / raid restent en auto-protection (inchangé).
--
-- On re-déclare la clé pour : (1) passer le défaut à false, (2) clarifier le
-- libellé/description. Idempotent.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'auto_delete_links_enabled'
        UNION ALL SELECT '{
            "key": "auto_delete_links_enabled",
            "label": "Supprimer SÈCHEMENT les liens génériques (sinon : carte)",
            "type": "boolean",
            "required": false,
            "default": "false",
            "description": "OFF (défaut) : un lien générique non autorisé hors image génère une carte de review (décision humaine). ON : suppression automatique immédiate sans carte (mode agressif). Le phishing et les invitations Discord restent traités en auto-protection quoi qu''il arrive."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'automod-bot';
