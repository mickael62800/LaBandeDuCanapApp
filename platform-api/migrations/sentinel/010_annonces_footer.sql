-- Annonces planifiees : texte de pied d'embed (`footer`).
-- Discord rend le footer SOUS l'image de l'embed : c'est la seule zone de
-- texte situee en dessous de l'image, d'ou "texte du haut" = description et
-- "texte du bas" = footer, le tout dans un seul message.
--
-- Le reglage `text_position` (image envoyee dans un message separe) n'est plus
-- lu : l'image est desormais integree a l'embed. La colonne est conservee pour
-- ne pas casser les lignes existantes, mais plus aucun code ne l'ecrit.

ALTER TABLE public.scheduled_announcements
    ADD COLUMN IF NOT EXISTS embed_footer_text text;

-- Discord limite le footer a 2048 caracteres.
ALTER TABLE public.scheduled_announcements
    DROP CONSTRAINT IF EXISTS scheduled_announcements_embed_footer_text_check;
ALTER TABLE public.scheduled_announcements
    ADD CONSTRAINT scheduled_announcements_embed_footer_text_check
    CHECK ((embed_footer_text IS NULL) OR (char_length(embed_footer_text) <= 2048));

-- La colonne n'est plus alimentee : un defaut suffit pour les insertions.
ALTER TABLE public.scheduled_announcements
    ALTER COLUMN text_position SET DEFAULT 'below';
