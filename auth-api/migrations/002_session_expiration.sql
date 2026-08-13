-- Expiration absolue des sessions OAuth, appliquee cote serveur.
--
-- Max-Age dans le cookie ne suffit pas : un client peut fabriquer lui-meme le
-- header Cookie et continuer a presenter un UUID ancien. PostgreSQL devient la
-- source de verite de la duree de vie. Les sessions deja presentes gardent une
-- fenetre de 30 jours calculee depuis leur creation, sans la prolonger.
ALTER TABLE web_oauth_sessions
    ADD COLUMN IF NOT EXISTS expires_at timestamp with time zone;

UPDATE web_oauth_sessions
SET expires_at = created_at + interval '30 days'
WHERE expires_at IS NULL;

ALTER TABLE web_oauth_sessions
    ALTER COLUMN expires_at SET DEFAULT (now() + interval '30 days'),
    ALTER COLUMN expires_at SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_web_oauth_sessions_expires_at
    ON web_oauth_sessions USING btree (expires_at);
