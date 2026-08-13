-- Bans IP manuels declenches depuis le panel securite (distincts des bans
-- fail2ban automatiques). Cette table sert de source de verite pour
-- afficher la liste, debannir, et filtrer les logs nginx-suspicious.
CREATE TABLE IF NOT EXISTS manual_ip_bans (
    ip            TEXT PRIMARY KEY,
    banned_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    banned_by     TEXT,
    reason        TEXT,
    unbanned_at   TIMESTAMPTZ,
    unbanned_by   TEXT
);

CREATE INDEX IF NOT EXISTS idx_manual_ip_bans_active
    ON manual_ip_bans (banned_at DESC)
    WHERE unbanned_at IS NULL;
