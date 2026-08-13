-- Phase 2 Annonces : boutons interactifs + reactions automatiques.
--
-- buttons : JSONB array de { label, style, custom_id?, url? }
--   style ∈ 'primary' | 'secondary' | 'success' | 'danger' | 'link'
--   - primary/secondary/success/danger -> custom_id obligatoire (action)
--   - link -> url obligatoire (ouvre le navigateur, pas d'interaction)
--   max 5 boutons par annonce (limite Discord = 5/row)
-- auto_reactions : JSONB array d'emojis (unicode ou <:name:id> custom)
--   max 20 reactions ajoutees automatiquement apres post

ALTER TABLE scheduled_announcements
    ADD COLUMN IF NOT EXISTS buttons JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS auto_reactions JSONB NOT NULL DEFAULT '[]'::jsonb;

-- Tracking des interactions sur les boutons. Permet de voir qui a clique
-- sur quel bouton et combien de fois (utile pour engagement / RSVP).
CREATE TABLE IF NOT EXISTS announcement_button_interactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    announcement_id UUID NOT NULL REFERENCES scheduled_announcements(id) ON DELETE CASCADE,
    run_id UUID,
    user_id TEXT NOT NULL,
    user_name TEXT,
    button_custom_id TEXT NOT NULL,
    button_label TEXT,
    clicked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_button_interactions_announcement
    ON announcement_button_interactions (announcement_id, clicked_at DESC);
CREATE INDEX IF NOT EXISTS idx_button_interactions_user
    ON announcement_button_interactions (user_id, clicked_at DESC);
