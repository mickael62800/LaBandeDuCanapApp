-- Rappels de sanctions temporaires (DM moderateur avant expiration)

CREATE TABLE sanction_reminders (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    moderator_id    TEXT NOT NULL,
    moderator_name  TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    target_name     TEXT NOT NULL,
    action_type     TEXT NOT NULL,
    reason          TEXT NOT NULL,
    action_id       UUID NOT NULL,
    remind_at       TIMESTAMPTZ NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reminders_pending ON sanction_reminders(remind_at) WHERE status = 'pending';
CREATE INDEX idx_reminders_action ON sanction_reminders(action_id);
