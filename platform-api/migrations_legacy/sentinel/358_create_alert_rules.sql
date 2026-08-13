-- Regles d'alerte configurables pour le dispatcher de supervision.
--
-- Avant : 3 regles codees en dur (auth failures > 50, conteneur supprime,
-- cert TLS < 14j) dans alerts_dispatcher.rs. Cette table rend les seuils
-- configurables et ajoute la supervision des ressources (CPU/RAM/disque) et
-- des services offline — donnees deja collectees mais jamais surveillees.
--
-- `metric` = discriminant evalue par le dispatcher (valeurs connues) :
--   cpu_percent, mem_percent, disk_percent, auth_failures_1h, tls_expiry_days
--   (seuils numeriques via comparator/threshold),
--   service_offline, container_removed (declencheurs booleens, threshold ignore).
-- `comparator` : 'gt' (superieur) ou 'lt' (inferieur).
-- `cooldown_secs` : anti-repetition, applique via une cle Redis a TTL.

CREATE TABLE IF NOT EXISTS alert_rules (
    id            TEXT PRIMARY KEY,
    label         TEXT NOT NULL,
    metric        TEXT NOT NULL,
    comparator    TEXT NOT NULL DEFAULT 'gt',
    threshold     DOUBLE PRECISION,
    enabled       BOOLEAN NOT NULL DEFAULT TRUE,
    severity      TEXT NOT NULL DEFAULT 'warning',
    cooldown_secs INTEGER NOT NULL DEFAULT 3600,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Regles par defaut (idempotent : ON CONFLICT DO NOTHING pour ne pas ecraser
-- des seuils ajustes par l'admin lors d'une re-execution de la migration).
INSERT INTO alert_rules (id, label, metric, comparator, threshold, severity, cooldown_secs) VALUES
    ('cpu_percent',       'CPU host eleve',              'cpu_percent',       'gt', 90,  'warning',  1800),
    ('mem_percent',       'RAM host elevee',            'mem_percent',       'gt', 90,  'warning',  1800),
    ('disk_percent',      'Disque presque plein',       'disk_percent',      'gt', 85,  'critical', 3600),
    ('auth_failures_1h',  'Echecs d''auth (brute-force)','auth_failures_1h', 'gt', 50,  'critical', 3600),
    ('service_offline',   'Service bot/worker offline', 'service_offline',   'gt', NULL,'critical', 1800),
    ('tls_expiry_days',   'Certificat TLS bientot expire','tls_expiry_days', 'lt', 14,  'warning',  86400),
    ('container_removed', 'Conteneur supprime/modifie', 'container_removed', 'gt', NULL,'warning',  3600)
ON CONFLICT (id) DO NOTHING;
