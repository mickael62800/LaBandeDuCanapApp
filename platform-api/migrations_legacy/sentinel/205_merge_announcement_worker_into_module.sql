-- Phase composants — Fusion du worker `announcement-worker` dans le
-- module `announcements`.
--
-- Avant : la page Composants affichait 2 entrees pour les annonces :
--   - "Annonces planifiees" (module : config metier)
--   - "Worker Annonces planifiees" (worker : config infra)
--
-- L'utilisateur devait activer/configurer les deux. Pour rien : si le
-- module est actif il faut bien que le worker tourne.
--
-- Apres : 1 seul module "announcements" avec config_schema fusionnee
-- (cles metier + cles worker). Les configs guild ont deja ete renommees
-- de `announcement-worker` vers `announcements` par la migration 204.
--
-- Idempotent : ON CONFLICT met a jour la definition.

-- 1) Etend le config_schema du module avec les cles infra du worker.
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'announcements',
    'Annonces planifiees',
    'Messages Discord postes automatiquement (ponctuel, quotidien, hebdo, mensuel) avec embed riche, mentions, boutons interactifs et reactions automatiques. Le timer de publication tourne dans sentinel-worker.',
    '[
        {"key": "default_color_hex", "label": "Couleur par defaut (embed) en hex (ex: #5865f2)", "type": "text", "required": false, "default": "#5865f2"},
        {"key": "max_announcements_per_guild", "label": "Nombre max d''annonces par serveur", "type": "number", "required": false, "default": "100"},
        {"key": "default_mention_everyone", "label": "Activer @everyone par defaut", "type": "boolean", "required": false, "default": "false"},
        {"key": "history_retention_days", "label": "Retention historique (jours)", "type": "number", "required": false, "default": "90"},
        {"key": "log_channel_id", "label": "Salon de logs (publication / erreurs)", "type": "channel", "required": false},
        {"key": "fetch_limit", "label": "Nombre max d''annonces fetchees par tick worker", "type": "number", "required": false, "default": "50"}
    ]'
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;

-- 2) Supprime la definition du worker — il n'apparait plus dans la
-- section "Workers" de la page Composants. Les configs guild
-- existantes ont deja ete renommees vers `announcements` (mig 204).
DELETE FROM bot_definitions WHERE bot_name = 'announcement-worker';
