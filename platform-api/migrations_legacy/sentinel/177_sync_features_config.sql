-- Phase Sync — Configs exposables sur les features synchronisees Discord <-> Web.
--
-- Ajoute aux schemas bot_definitions les parametres pertinents pour les
-- features livrees en sync bilaterale (cf. SYNC_POCKET_GUIDE.md). Les valeurs
-- sont lues par le bot via `BaseApiClient::config_*` et editables depuis
-- la page web `/component-config`.

-- ── automod-bot : pile de review (reviews en attente) ──
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT jsonb_array_elements(config_schema) AS elem
        UNION ALL SELECT '{
            "key": "review_auto_resolve_after_hours",
            "label": "Auto-ignore reviews apres N heures",
            "type": "number",
            "required": false,
            "default": "0",
            "description": "Si > 0, les cartes de review pending sont automatiquement passees a status=ignored apres ce delai (anti-pileup). 0 = desactive (la pile peut grandir indefiniment).",
            "unit": "heures",
            "min": 0,
            "max": 720
        }'::jsonb
        UNION ALL SELECT '{
            "key": "review_min_score",
            "label": "Score IA minimum pour declencher une review",
            "type": "number",
            "required": false,
            "default": "0.0",
            "description": "En dessous de ce score, le bot applique l action automatiquement sans poster de carte de review. Au dessus, il poste une carte que la web peut resoudre. 0.0 = toutes les detections passent par la review.",
            "min": 0.0,
            "max": 10.0
        }'::jsonb
    ) sub
)
WHERE bot_name = 'automod-bot';

-- ── voice-bot : panneau de controle synchronise ──
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT jsonb_array_elements(config_schema) AS elem
        UNION ALL SELECT '{
            "key": "panel_post_enabled",
            "label": "Poster le panneau de controle dans le chat vocal",
            "type": "boolean",
            "required": false,
            "default": "true",
            "description": "Si OFF, aucun panneau de controle n est poste a la creation d un salon vocal temporaire. Les actions admin restent disponibles via la page /voice-channels (la sync bilaterale est sans objet sans panneau)."
        }'::jsonb
        UNION ALL SELECT '{
            "key": "panel_sync_grace_secs",
            "label": "Delai avant grise du panneau apres modif web",
            "type": "number",
            "required": false,
            "default": "0",
            "description": "Delai cosmetique en secondes avant que le bot n edite le panneau Discord apres une modif depuis la web (laisse le temps a Discord de propager). 0 = immediat.",
            "unit": "secondes",
            "min": 0,
            "max": 30
        }'::jsonb
    ) sub
)
WHERE bot_name = 'voice-bot';

-- ── blackjack-bot : tables multijoueur ──
-- max_players_per_table existe deja. Ajoute le grace period a la fermeture.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT jsonb_array_elements(config_schema) AS elem
        UNION ALL SELECT '{
            "key": "table_close_notify_players",
            "label": "DM les joueurs quand une table est fermee depuis la web",
            "type": "boolean",
            "required": false,
            "default": "false",
            "description": "Si ON, le bot envoie un DM aux joueurs presents a la table quand un admin la ferme depuis la web admin (transparence). Si OFF, seul l embed Discord est mis a jour."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'blackjack-bot';

-- ── coude-bot : combats annulables depuis la web ──
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT jsonb_array_elements(config_schema) AS elem
        UNION ALL SELECT '{
            "key": "combat_web_cancel_refund",
            "label": "Rembourser la mise a l attaquant si combat annule via web",
            "type": "boolean",
            "required": false,
            "default": "true",
            "description": "Si ON, quand un admin annule un defi pending depuis la web admin, la mise prelevee a l attaquant est creditee a son wallet. Si OFF, la mise est perdue (mode strict / sanction)."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'coude-bot';
