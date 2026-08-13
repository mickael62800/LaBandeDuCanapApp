-- Embed builder (style Carl-bot) : embeds nommes, configurables, postables et
-- editables. Les fields (name/value/inline) sont stockes en JSONB.

CREATE TABLE IF NOT EXISTS public.embeds (
    id uuid DEFAULT gen_random_uuid() NOT NULL PRIMARY KEY,
    guild_id text NOT NULL,
    name text NOT NULL,
    -- Contenu hors embed (message texte au-dessus de la carte).
    content text NOT NULL DEFAULT '',
    -- Author (haut de l'embed).
    author_name text NOT NULL DEFAULT '',
    author_icon_url text NOT NULL DEFAULT '',
    author_url text NOT NULL DEFAULT '',
    -- Corps.
    title text NOT NULL DEFAULT '',
    title_url text NOT NULL DEFAULT '',
    description text NOT NULL DEFAULT '',
    color integer,
    image_url text NOT NULL DEFAULT '',
    thumbnail_url text NOT NULL DEFAULT '',
    -- Footer (bas de l'embed).
    footer_text text NOT NULL DEFAULT '',
    footer_icon_url text NOT NULL DEFAULT '',
    show_timestamp boolean NOT NULL DEFAULT false,
    -- Champs : [{ "name": "...", "value": "...", "inline": true }]
    fields jsonb NOT NULL DEFAULT '[]'::jsonb,
    -- Dernier message poste (pour l'edition a la volee).
    last_channel_id text,
    last_message_id text,
    created_by text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS embeds_guild_id_idx ON public.embeds (guild_id);
