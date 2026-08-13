-- Annonces planifiees : recurrence ANNUELLE (une fois par an, ex. saisonnier)
-- + position du texte par rapport a l'image (image envoyee en message separe).

ALTER TABLE public.scheduled_announcements
    ADD COLUMN IF NOT EXISTS recurrence_month smallint,
    ADD COLUMN IF NOT EXISTS text_position text NOT NULL DEFAULT 'below';

-- Autoriser 'yearly' comme type de recurrence.
ALTER TABLE public.scheduled_announcements
    DROP CONSTRAINT IF EXISTS scheduled_announcements_recurrence_type_check;
ALTER TABLE public.scheduled_announcements
    ADD CONSTRAINT scheduled_announcements_recurrence_type_check
    CHECK ((recurrence_type = ANY (ARRAY['once'::text, 'daily'::text, 'weekly'::text, 'monthly'::text, 'yearly'::text])));

-- Coherence : 'yearly' exige un mois ET un jour du mois.
ALTER TABLE public.scheduled_announcements
    DROP CONSTRAINT IF EXISTS recurrence_consistency;
ALTER TABLE public.scheduled_announcements
    ADD CONSTRAINT recurrence_consistency CHECK (
        (((recurrence_type = 'once'::text) AND (scheduled_at IS NOT NULL))
        OR (recurrence_type = 'daily'::text)
        OR ((recurrence_type = 'weekly'::text) AND (recurrence_day_of_week IS NOT NULL))
        OR ((recurrence_type = 'monthly'::text) AND (recurrence_day_of_month IS NOT NULL))
        OR ((recurrence_type = 'yearly'::text) AND (recurrence_day_of_month IS NOT NULL) AND (recurrence_month IS NOT NULL)))
    );

-- Bornes du mois (1-12) et de la position.
ALTER TABLE public.scheduled_announcements
    ADD CONSTRAINT scheduled_announcements_recurrence_month_check
    CHECK ((recurrence_month IS NULL) OR ((recurrence_month >= 1) AND (recurrence_month <= 12)));
ALTER TABLE public.scheduled_announcements
    ADD CONSTRAINT scheduled_announcements_text_position_check
    CHECK ((text_position = ANY (ARRAY['above'::text, 'below'::text])));
