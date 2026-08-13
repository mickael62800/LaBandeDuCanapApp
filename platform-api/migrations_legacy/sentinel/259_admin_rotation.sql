-- Administrateur tournant — un modo devient admin a tour de role chaque
-- periode (mois par defaut), apres acceptation du modo + validation de l'owner.
--
-- Etat de la machine (par guild) :
--   idle               : aucune rotation en cours
--   offering_candidate : MP envoye a un candidat, en attente de sa reponse
--   awaiting_owner     : candidat a accepte, en attente de validation owner
--   offering_stay      : tous ont refuse, on demande a l'admin actuel de rester

CREATE TABLE IF NOT EXISTS admin_rotation (
    guild_id TEXT PRIMARY KEY,
    state TEXT NOT NULL DEFAULT 'idle'
        CHECK (state IN ('idle','offering_candidate','awaiting_owner','offering_stay')),
    -- Admin tournant en cours (rôle attribue).
    current_admin_id TEXT,
    current_admin_since TIMESTAMPTZ,
    -- Periode courante.
    period_start TIMESTAMPTZ,
    next_rotation_at TIMESTAMPTZ,
    -- Candidat en cours de sollicitation + horodatage (pour le timeout).
    candidate_id TEXT,
    candidate_offered_at TIMESTAMPTZ,
    -- IDs deja sollicites durant CETTE rotation (pour ne pas reproposer).
    asked_this_round JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Historique : qui a ete admin et quand (round-robin = on ressert le plus
-- ancien / jamais servi).
CREATE TABLE IF NOT EXISTS admin_rotation_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    served_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_admin_rotation_history_guild
    ON admin_rotation_history (guild_id, served_at DESC);

-- Definition du bot + config (page Composants).
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'rotation-bot',
    'Administrateur tournant',
    'Chaque periode, un moderateur devient administrateur a tour de role (acceptation du modo + validation de l owner).',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "false", "description": "Active la rotation automatique de l administrateur."},
        {"key": "mod_role_id", "label": "Role Moderateur (pool)", "type": "role", "required": false, "description": "Les membres ayant ce role sont les candidats a la rotation.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "admin_role_id", "label": "Role Administrateur (attribue)", "type": "role", "required": false, "description": "Role donne au modo selectionne (et retire au precedent, qui redevient Moderateur).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "period_days", "label": "Duree d un mandat", "type": "number", "required": false, "default": "30", "min": 1, "max": 366, "unit": "jours", "description": "Au bout de cette duree, on lance une nouvelle rotation.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "response_timeout_hours", "label": "Delai de reponse", "type": "number", "required": false, "default": "72", "min": 1, "max": 720, "unit": "heures", "description": "Temps laisse au modo (et a l owner) pour repondre avant de passer au suivant.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "objective_message", "label": "Message / objectif (MP au modo)", "type": "text", "required": false, "default": "Ce mois-ci, c est ton tour de devenir Administrateur ! Ton objectif : animer le serveur, veiller au respect des regles et accompagner la communaute. Acceptes-tu ce mandat ?", "description": "Texte envoye en MP au candidat. Tu peux expliquer son objectif/mandat.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
)
ON CONFLICT (bot_name) DO UPDATE
    SET display_name = EXCLUDED.display_name, description = EXCLUDED.description;
