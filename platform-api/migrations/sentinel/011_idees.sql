-- Module "Idees" : boite a idees du serveur, calquee sur les tickets.
--
-- Un membre propose une idee depuis un panneau (bouton + modale). Le bot cree
-- un salon prive dedie ou l'auteur et le staff affinent la proposition, avec
-- des boutons de statut reserves au staff (en discussion / acceptee / refusee
-- / realisee). Les echanges sont synchronises dans `idea_messages` pour etre
-- relus depuis le web, comme pour les tickets.

CREATE TABLE IF NOT EXISTS public.ideas (
    id uuid DEFAULT gen_random_uuid() NOT NULL PRIMARY KEY,
    guild_id text NOT NULL,
    title text NOT NULL,
    description text NOT NULL DEFAULT '',
    -- nouvelle | en_discussion | acceptee | refusee | realisee
    status text NOT NULL DEFAULT 'nouvelle',
    -- Categorie libre choisie dans la modale (evenement, salon, role, ...).
    category text NOT NULL DEFAULT 'autre',
    author_id character varying(20) NOT NULL,
    author_name text NOT NULL,
    -- Salon prive dedie a l'idee (NULL si sa creation a echoue).
    channel_id character varying(20),
    -- Decision du staff : qui, pourquoi, quand.
    decided_by character varying(20),
    decided_by_name text,
    decision_reason text,
    decided_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ideas_status_check CHECK (
        status = ANY (ARRAY[
            'nouvelle'::text, 'en_discussion'::text,
            'acceptee'::text, 'refusee'::text, 'realisee'::text
        ])
    )
);

CREATE INDEX IF NOT EXISTS ideas_guild_id_idx ON public.ideas (guild_id);
CREATE INDEX IF NOT EXISTS ideas_status_idx ON public.ideas (guild_id, status);
CREATE INDEX IF NOT EXISTS ideas_author_id_idx ON public.ideas (author_id);
-- Un seul salon Discord par idee (et reciproquement) : evite qu'un double
-- clic ou un rejeu d'event rattache deux idees au meme salon.
CREATE UNIQUE INDEX IF NOT EXISTS ideas_channel_id_key
    ON public.ideas (channel_id) WHERE channel_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS public.idea_messages (
    id uuid DEFAULT gen_random_uuid() NOT NULL PRIMARY KEY,
    idea_id uuid NOT NULL REFERENCES public.ideas (id) ON DELETE CASCADE,
    author_name text NOT NULL,
    -- auteur | staff
    author_role text NOT NULL DEFAULT 'auteur',
    content text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idea_messages_idea_id_idx
    ON public.idea_messages (idea_id, created_at);

-- Declaration du module pour la page de configuration web (bot_guild_config).
INSERT INTO public.bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'idea-bot',
    'Idees',
    'Boite a idees : les membres proposent, le staff tranche',
    '[
      {"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false,
       "description": "Active la boite a idees du serveur."},
      {"key": "panel_channel_id", "type": "channel", "label": "Salon du panneau", "required": true,
       "depends_on": {"key": "enabled", "equals": "true"},
       "description": "Salon ou le panneau Proposer une idee est poste."},
      {"key": "idea_category_id", "type": "category", "label": "Categorie des salons idees", "required": false,
       "depends_on": {"key": "enabled", "equals": "true"},
       "description": "Categorie Discord ou les salons prives d idees sont crees. Vide = pas de categorie."},
      {"key": "staff_role_id", "type": "role", "label": "Role staff", "required": true,
       "depends_on": {"key": "enabled", "equals": "true"},
       "description": "Role autorise a changer le statut d une idee et a voir les salons."},
      {"key": "max_open_per_user", "type": "number", "min": 0, "max": 50, "label": "Max idees ouvertes par membre",
       "default": "3", "required": false, "depends_on": {"key": "enabled", "equals": "true"},
       "description": "0 = illimite. Compte les idees non tranchees (nouvelle / en discussion)."},
      {"key": "welcome_message", "type": "text", "label": "Message d accueil du salon", "default": "",
       "required": false, "depends_on": {"key": "enabled", "equals": "true"},
       "description": "Vide = message par defaut. Poste dans le salon de l idee a sa creation."},
      {"key": "archive_delay_secs", "type": "number", "min": 0, "max": 86400, "unit": "s",
       "label": "Delai avant suppression du salon", "default": "60", "required": false,
       "depends_on": {"key": "enabled", "equals": "true"},
       "description": "Apres une decision (acceptee / refusee / realisee), on attend N secondes avant de supprimer le salon. 0 = suppression immediate."},
      {"key": "title_min_len", "type": "number", "min": 1, "max": 4000, "unit": "caracteres",
       "label": "Titre — longueur min", "default": "5", "required": false},
      {"key": "title_max_len", "type": "number", "min": 1, "max": 4000, "unit": "caracteres",
       "label": "Titre — longueur max", "default": "100", "required": false},
      {"key": "desc_min_len", "type": "number", "min": 1, "max": 4000, "unit": "caracteres",
       "label": "Description — longueur min", "default": "20", "required": false},
      {"key": "desc_max_len", "type": "number", "min": 1, "max": 4000, "unit": "caracteres",
       "label": "Description — longueur max", "default": "2000", "required": false},
      {"key": "color_new", "type": "text", "label": "Couleur idee nouvelle (hex)", "default": "3498db", "required": false},
      {"key": "color_accepted", "type": "text", "label": "Couleur idee acceptee (hex)", "default": "2ecc71", "required": false},
      {"key": "color_refused", "type": "text", "label": "Couleur idee refusee (hex)", "default": "e74c3c", "required": false},
      {"key": "color_done", "type": "text", "label": "Couleur idee realisee (hex)", "default": "9b59b6", "required": false}
    ]'::jsonb
)
ON CONFLICT (bot_name) DO NOTHING;
