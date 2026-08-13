-- Welcome — le "role apres validation du reglement" devient une LISTE de roles.
--
-- Avant : `rules_role_id` etait un selecteur de role unique (type "role").
-- Desormais on veut pouvoir attribuer PLUSIEURS roles d'un coup (ex. role
-- "Information" + role "Jeu"). On passe le champ en type "text" : le dashboard
-- l'affiche alors comme un multi-picker de roles (cf. ROLE_LIST_KEYS cote web)
-- qui stocke un CSV d'IDs. Une valeur existante (un seul ID) reste un CSV
-- valide a 1 element -> retro-compatible.

-- Idempotent : on retire d'abord la cle si presente, puis on la (re)ajoute.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'rules_role_id'
        UNION ALL SELECT '{
            "key": "rules_role_id",
            "label": "Roles apres validation",
            "type": "text",
            "required": false,
            "description": "Roles attribues quand un membre clique sur le bouton d acceptation. Tu peux en choisir plusieurs."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'welcome-bot';
