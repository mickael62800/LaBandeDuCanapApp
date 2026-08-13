-- Phase 3 (refacto journal d'audit) : backfill audit_logs depuis les anciennes
-- tables moderation_actions et security_events.
--
-- Idempotent : on filtre sur (event_type LIKE ...) AND NOT EXISTS pour ne pas
-- recreer les lignes qu'un dual-write aurait deja inserees.

-- ── moderation_actions → audit_logs ──
INSERT INTO audit_logs (
    id, guild_id, event_type, actor_id, actor_name,
    target_id, target_name, channel_id, channel_name, details, created_at
)
SELECT
    gen_random_uuid(),
    ma.guild_id,
    'mod_' || ma.action_type,
    ma.moderator_id,
    ma.moderator_name,
    ma.target_id,
    ma.target_name,
    ma.channel_id,
    NULL,
    jsonb_build_object(
        'reason', ma.reason,
        'gravity', ma.gravity,
        'duration_secs', ma.duration,
        'action_id', ma.id::text,
        'backfilled', true
    ),
    ma.created_at
FROM moderation_actions ma
WHERE NOT EXISTS (
    SELECT 1 FROM audit_logs al
    WHERE al.event_type = 'mod_' || ma.action_type
      AND al.target_id = ma.target_id
      AND al.created_at = ma.created_at
);

-- ── security_events → audit_logs ──
-- Si user_ids contient un seul user, on l'utilise comme target.
INSERT INTO audit_logs (
    id, guild_id, event_type, actor_id, actor_name,
    target_id, target_name, channel_id, channel_name, details, created_at
)
SELECT
    gen_random_uuid(),
    se.guild_id,
    'security_' || se.event_type,
    NULL,
    NULL,
    CASE
        WHEN jsonb_array_length(se.user_ids) = 1
            THEN se.user_ids->>0
        ELSE NULL
    END,
    CASE
        WHEN jsonb_array_length(se.user_ids) = 1
            THEN se.user_ids->>0
        ELSE NULL
    END,
    NULL,
    NULL,
    jsonb_build_object(
        'severity', se.severity,
        'description', se.description,
        'user_ids', se.user_ids,
        'event_id', se.id::text,
        'backfilled', true
    ),
    se.created_at
FROM security_events se
WHERE NOT EXISTS (
    SELECT 1 FROM audit_logs al
    WHERE al.event_type = 'security_' || se.event_type
      AND al.created_at = se.created_at
      AND (al.details->>'event_id' = se.id::text OR al.details->>'event_id' IS NULL)
);
