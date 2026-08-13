-- Automod — auto-protection des cas severes (raid / phishing / pub Discord / gros flood).
--
-- Meme en mode "modération 100% humaine" (human_only_enabled), certains cas ne
-- peuvent pas attendre une décision humaine : raid, spam d'invitations Discord,
-- liens de phishing, gros flood. Pour ceux-là le bot applique immédiatement une
-- mesure REVERSIBLE (mute/timeout + suppression du message) puis poste TOUJOURS
-- une carte de review pour que les modérateurs valident ou ajustent.
--
-- Deux cles exposees dans la page web :
--   - auto_protect_enabled        : active/desactive cette protection auto.
--   - severe_flood_max_messages   : seuil de "gros flood" (nb de messages dans
--                                   la fenetre flood) qui declenche l'auto-protection.
--
-- Idempotent : on retire d'abord les cles si presentes, puis on les (re)ajoute.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' NOT IN ('auto_protect_enabled', 'severe_flood_max_messages')
        UNION ALL SELECT '{
            "key": "auto_protect_enabled",
            "label": "Auto-protection des cas severes (raid / phishing / pub / gros flood)",
            "type": "boolean",
            "required": false,
            "default": "true",
            "description": "Si ON, les cas severes (phishing, invitation Discord, gros flood) declenchent une mesure reversible immediate (mute + suppression) MEME en moderation 100% humaine, puis une carte de review est toujours postee pour validation/ajustement par un moderateur."
        }'::jsonb
        UNION ALL SELECT '{
            "key": "severe_flood_max_messages",
            "label": "Seuil gros flood (messages) pour auto-protection",
            "type": "number",
            "required": false,
            "default": "12",
            "description": "Nombre de messages dans la fenetre de flood au-dela duquel on considere un gros flood / raid et on declenche l''auto-protection. Doit etre >= au seuil de flood simple."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'automod-bot';
