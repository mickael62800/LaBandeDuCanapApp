--
-- 001_init.sql — schema initial consolide de DiscordSentinel
--
-- Ce fichier remplace les 370 migrations historiques (archivees dans
-- sentinel-api/migrations_legacy/). Il est destine a une base VIERGE
-- uniquement : un deploiement existant doit etre recree from scratch
-- (drop de la base puis application de cette migration).
--
-- Contenu, dans l'ordre : extension, types, fonctions, tables (+ partitions),
-- vues materialisees, sequences/defaults, contraintes, index, triggers,
-- puis seeds (alert_rules, bot_definitions).
--
-- Index strictement redondants supprimes lors du squash (doublons exacts
-- de contraintes UNIQUE existantes) :
--   - idx_confessions_guild (= confessions_guild_id_public_number_key)
--   - idx_user_stats_guild_user (= uq_user_stats_guild_user)
--   - idx_confession_replies_confession (= confession_replies_confession_id_public_number_key)
--   - idx_automod_discussion_channels_review (= automod_discussion_channels_review_id_key)
--   - idx_daily_activity_guild_day (= uq_daily_activity_guild_day)
--

--
--


--
-- Schema initial consolide de DiscordSentinel
--


--
-- Name: moderation_gravity; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.moderation_gravity AS ENUM (
    'low',
    'medium',
    'high',
    'critical'
);


--
-- Name: voice_channel_kind; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.voice_channel_kind AS ENUM (
    'public',
    'private'
);


--
-- Name: enrich_schema_keys(text, jsonb); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.enrich_schema_keys(p_bot_name text, p_patch jsonb) RETURNS void
    LANGUAGE plpgsql
    AS $$
DECLARE
    v_schema JSONB;
    v_new_schema JSONB := '[]'::jsonb;
    v_entry JSONB;
    v_key TEXT;
    v_overrides JSONB;
BEGIN
    SELECT config_schema INTO v_schema FROM bot_definitions WHERE bot_name = p_bot_name;
    IF v_schema IS NULL THEN
        RAISE NOTICE 'enrich_schema_keys: bot % introuvable, skip', p_bot_name;
        RETURN;
    END IF;

    FOR v_entry IN SELECT * FROM jsonb_array_elements(v_schema)
    LOOP
        v_key := v_entry->>'key';
        v_overrides := p_patch->v_key;
        IF v_overrides IS NOT NULL THEN
            -- Merge : les champs du patch ecrasent ceux de l entree existante.
            v_entry := v_entry || v_overrides;
        END IF;
        v_new_schema := v_new_schema || jsonb_build_array(v_entry);
    END LOOP;

    UPDATE bot_definitions SET config_schema = v_new_schema WHERE bot_name = p_bot_name;
END;
$$;


--
-- Name: admin_rotation; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.admin_rotation (
    guild_id text NOT NULL,
    state text DEFAULT 'idle'::text NOT NULL,
    current_admin_id text,
    current_admin_since timestamp with time zone,
    period_start timestamp with time zone,
    next_rotation_at timestamp with time zone,
    candidate_id text,
    candidate_offered_at timestamp with time zone,
    asked_this_round jsonb DEFAULT '[]'::jsonb NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT admin_rotation_state_check CHECK ((state = ANY (ARRAY['idle'::text, 'offering_candidate'::text, 'awaiting_owner'::text, 'offering_stay'::text])))
);


--
-- Name: admin_rotation_history; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.admin_rotation_history (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id text NOT NULL,
    user_id text NOT NULL,
    served_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: age_verification_bans; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.age_verification_bans (
    id uuid NOT NULL,
    guild_id text NOT NULL,
    user_id text NOT NULL,
    declared_age integer NOT NULL,
    banned_at timestamp with time zone DEFAULT now() NOT NULL,
    unban_at timestamp with time zone NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    lifted_at timestamp with time zone
);


--
-- Name: ai_dataset_messages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ai_dataset_messages (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id text NOT NULL,
    channel_id text,
    channel_name text,
    user_id text NOT NULL,
    content text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: ai_jobs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ai_jobs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    job_type text NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    input_payload jsonb NOT NULL,
    result_payload jsonb,
    error_message text,
    retries integer DEFAULT 0 NOT NULL,
    max_retries integer DEFAULT 3 NOT NULL,
    cost_tokens bigint DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    started_at timestamp with time zone,
    completed_at timestamp with time zone,
    CONSTRAINT chk_ai_jobs_status CHECK ((status = ANY (ARRAY['pending'::text, 'processing'::text, 'done'::text, 'failed'::text, 'dead'::text]))),
    CONSTRAINT chk_ai_jobs_type CHECK ((job_type = ANY (ARRAY['analyze_text'::text, 'analyze_image'::text])))
);


--
-- Name: alert_rules; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.alert_rules (
    id text NOT NULL,
    label text NOT NULL,
    metric text NOT NULL,
    comparator text DEFAULT 'gt'::text NOT NULL,
    threshold double precision,
    enabled boolean DEFAULT true NOT NULL,
    severity text DEFAULT 'warning'::text NOT NULL,
    cooldown_secs integer DEFAULT 3600 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: analytics_daily_baseline; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.analytics_daily_baseline (
    guild_id text NOT NULL,
    day date NOT NULL,
    total_messages bigint DEFAULT 0 NOT NULL,
    total_voice_seconds bigint DEFAULT 0 NOT NULL,
    captured_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: announcement_button_interactions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.announcement_button_interactions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    announcement_id uuid NOT NULL,
    run_id uuid,
    user_id text NOT NULL,
    user_name text,
    button_custom_id text NOT NULL,
    button_label text,
    clicked_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: api_user_guilds; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.api_user_guilds (
    discord_user_id character varying(20) NOT NULL,
    guild_id character varying(20) NOT NULL,
    role text NOT NULL,
    granted_at timestamp with time zone DEFAULT now() NOT NULL,
    granted_by character varying(20),
    CONSTRAINT api_user_guilds_role_check CHECK ((role = ANY (ARRAY['owner'::text, 'admin'::text, 'moderator'::text, 'viewer'::text])))
);


--
-- Name: api_users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.api_users (
    discord_user_id character varying(20) NOT NULL,
    display_name text NOT NULL,
    avatar_url text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
)
PARTITION BY RANGE (created_at);


--
-- Name: audit_logs_2026_04; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2026_04 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_2026_05; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2026_05 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_2026_06; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2026_06 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_2026_07; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2026_07 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_2026_08; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2026_08 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_2026_09; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2026_09 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_2026_10; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2026_10 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_2026_11; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2026_11 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_2026_12; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2026_12 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_2027_01; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2027_01 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_2027_02; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2027_02 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_2027_03; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_2027_03 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: audit_logs_default; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs_default (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    actor_id character varying(20),
    actor_name text,
    target_id character varying(20),
    target_name text,
    channel_id character varying(20),
    channel_name text,
    details jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: auto_roles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.auto_roles (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    role_id character varying(20) NOT NULL,
    role_name text DEFAULT ''::text NOT NULL,
    delay_secs integer DEFAULT 0 NOT NULL,
    enabled boolean DEFAULT true NOT NULL
);


--
-- Name: automod_adaptive_slowmode; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.automod_adaptive_slowmode (
    channel_id text NOT NULL,
    guild_id text NOT NULL,
    activated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: automod_discussion_channels; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.automod_discussion_channels (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    review_id uuid NOT NULL,
    guild_id text NOT NULL,
    channel_id text NOT NULL,
    opened_by_id text NOT NULL,
    opened_by_name text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: automod_discussion_messages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.automod_discussion_messages (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    review_id uuid NOT NULL,
    discord_message_id text NOT NULL,
    author_id text NOT NULL,
    author_name text DEFAULT ''::text NOT NULL,
    author_is_bot boolean DEFAULT false NOT NULL,
    content text DEFAULT ''::text NOT NULL,
    sent_at timestamp with time zone NOT NULL,
    captured_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: automod_review_votes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.automod_review_votes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    review_id uuid NOT NULL,
    voter_id text NOT NULL,
    voter_name text DEFAULT ''::text NOT NULL,
    vote_action text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT automod_review_votes_vote_action_check CHECK ((vote_action = ANY (ARRAY['prevention'::text, 'warn'::text, 'delete'::text, 'mute'::text, 'ban'::text, 'ignore'::text])))
);


--
-- Name: automod_reviews; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.automod_reviews (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id text NOT NULL,
    channel_id text NOT NULL,
    message_id text NOT NULL,
    user_id text NOT NULL,
    user_name text NOT NULL,
    content_preview text NOT NULL,
    suggested_action text NOT NULL,
    score double precision DEFAULT 0 NOT NULL,
    reason text DEFAULT ''::text NOT NULL,
    flags jsonb DEFAULT '{}'::jsonb NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    applied_action text,
    resolved_by_id text,
    resolved_by_name text,
    resolved_source text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    resolved_at timestamp with time zone,
    voting_deadline timestamp with time zone,
    decided_action text,
    quorum_met boolean DEFAULT false NOT NULL,
    decided_at timestamp with time zone,
    incident_count integer DEFAULT 1 NOT NULL,
    cumulative_score double precision DEFAULT 0 NOT NULL,
    incidents jsonb DEFAULT '[]'::jsonb NOT NULL,
    last_incident_at timestamp with time zone DEFAULT now() NOT NULL,
    sanction_logged boolean DEFAULT false NOT NULL,
    CONSTRAINT automod_reviews_applied_action_check CHECK (((applied_action IS NULL) OR (applied_action = ANY (ARRAY['prevention'::text, 'warn'::text, 'delete'::text, 'mute'::text, 'ban'::text, 'ignore'::text])))),
    CONSTRAINT automod_reviews_decided_action_check CHECK (((decided_action IS NULL) OR (decided_action = ANY (ARRAY['prevention'::text, 'warn'::text, 'delete'::text, 'mute'::text, 'ban'::text, 'ignore'::text])))),
    CONSTRAINT automod_reviews_resolved_source_check CHECK ((resolved_source = ANY (ARRAY['discord'::text, 'web'::text]))),
    CONSTRAINT automod_reviews_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'voting'::text, 'decided'::text, 'applied'::text, 'ignored'::text]))),
    CONSTRAINT automod_reviews_suggested_action_check CHECK ((suggested_action = ANY (ARRAY['warn'::text, 'delete'::text, 'mute'::text, 'ban'::text])))
);


--
-- Name: bot_definitions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bot_definitions (
    bot_name character varying(50) NOT NULL,
    display_name character varying(100) NOT NULL,
    description text DEFAULT ''::text NOT NULL,
    config_schema jsonb DEFAULT '[]'::jsonb NOT NULL
);


--
-- Name: bot_guild_config; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bot_guild_config (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    bot_name character varying(50) NOT NULL,
    config_key character varying(100) NOT NULL,
    config_value text NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: bump_events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bump_events (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id text NOT NULL,
    user_id text NOT NULL,
    username text DEFAULT ''::text NOT NULL,
    reward_coins integer DEFAULT 0 NOT NULL,
    weekly_index integer DEFAULT 1 NOT NULL,
    bumped_at timestamp with time zone DEFAULT now() NOT NULL,
    provider text DEFAULT 'disboard'::text NOT NULL
);


--
-- Name: bump_guild_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bump_guild_state (
    guild_id text NOT NULL,
    channel_id text DEFAULT ''::text NOT NULL,
    last_bump_at timestamp with time zone DEFAULT now() NOT NULL,
    cooldown_minutes integer DEFAULT 120 NOT NULL,
    reminder_enabled boolean DEFAULT true NOT NULL,
    reminder_sent boolean DEFAULT false NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    provider text DEFAULT 'disboard'::text NOT NULL
);


--
-- Name: confession_config; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.confession_config (
    guild_id text NOT NULL,
    enabled boolean DEFAULT true NOT NULL,
    channel_id text,
    panel_message_id text,
    cooldown_secs integer DEFAULT 60 NOT NULL,
    max_per_day integer DEFAULT 20 NOT NULL,
    min_chars integer DEFAULT 5 NOT NULL,
    max_chars integer DEFAULT 2000 NOT NULL,
    automod_enabled boolean DEFAULT true NOT NULL,
    banned_user_ids jsonb DEFAULT '[]'::jsonb NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    quota_window_hours integer DEFAULT 24 NOT NULL
);


--
-- Name: confession_counters; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.confession_counters (
    guild_id text NOT NULL,
    last_number integer DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: confession_replies; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.confession_replies (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    confession_id uuid NOT NULL,
    public_number integer NOT NULL,
    author_user_id text NOT NULL,
    content text NOT NULL,
    is_anonymous boolean DEFAULT true NOT NULL,
    message_id text,
    deleted_at timestamp with time zone,
    deleted_by text,
    edited_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: confession_reports; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.confession_reports (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id text NOT NULL,
    confession_id uuid,
    reply_id uuid,
    reporter_user_id text NOT NULL,
    reason text NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    resolved_by text,
    resolved_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT confession_reports_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'resolved'::text, 'dismissed'::text]))),
    CONSTRAINT report_target_required CHECK (((confession_id IS NOT NULL) OR (reply_id IS NOT NULL)))
);


--
-- Name: confessions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.confessions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id text NOT NULL,
    public_number integer NOT NULL,
    author_user_id text NOT NULL,
    content text NOT NULL,
    message_id text,
    channel_id text,
    thread_id text,
    deleted_at timestamp with time zone,
    deleted_by text,
    deleted_reason text,
    edited_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: daily_activity; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.daily_activity (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    day date NOT NULL,
    messages bigint DEFAULT 0 NOT NULL,
    voice_minutes bigint DEFAULT 0 NOT NULL,
    active_members integer DEFAULT 0 NOT NULL,
    new_members integer DEFAULT 0 NOT NULL,
    infractions integer DEFAULT 0 NOT NULL,
    warns integer DEFAULT 0 NOT NULL,
    mutes integer DEFAULT 0 NOT NULL,
    bans integer DEFAULT 0 NOT NULL,
    leaves integer DEFAULT 0 NOT NULL
);


--
-- Name: discord_action_messages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.discord_action_messages (
    action_id uuid NOT NULL,
    kind text NOT NULL,
    guild_id text NOT NULL,
    channel_id text NOT NULL,
    message_id text NOT NULL,
    posted_at timestamp with time zone DEFAULT now() NOT NULL,
    last_edited_at timestamp with time zone
);


--
-- Name: discord_audit_sync_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.discord_audit_sync_state (
    guild_id character varying(20) NOT NULL,
    last_entry_id text,
    last_synced_at timestamp with time zone DEFAULT now() NOT NULL,
    last_error text,
    consecutive_errors integer DEFAULT 0 NOT NULL
);


--
-- Name: discord_roles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.discord_roles (
    id text NOT NULL,
    guild_id character varying(20) NOT NULL,
    name text NOT NULL,
    color integer DEFAULT 0 NOT NULL,
    "position" integer DEFAULT 0 NOT NULL,
    permissions bigint DEFAULT 0 NOT NULL,
    mentionable boolean DEFAULT false NOT NULL,
    managed boolean DEFAULT false NOT NULL,
    icon text,
    member_count integer DEFAULT 0 NOT NULL,
    synced_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: export_jobs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.export_jobs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    requested_by character varying(20) NOT NULL,
    job_type text NOT NULL,
    format text NOT NULL,
    filters jsonb DEFAULT '{}'::jsonb NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    result text,
    result_rows integer,
    error_message text,
    retries integer DEFAULT 0 NOT NULL,
    max_retries integer DEFAULT 3 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    started_at timestamp with time zone,
    completed_at timestamp with time zone,
    CONSTRAINT chk_export_jobs_format CHECK ((format = ANY (ARRAY['csv'::text, 'json'::text]))),
    CONSTRAINT chk_export_jobs_status CHECK ((status = ANY (ARRAY['pending'::text, 'processing'::text, 'done'::text, 'failed'::text, 'dead'::text]))),
    CONSTRAINT chk_export_jobs_type CHECK ((job_type = ANY (ARRAY['infractions'::text, 'audit_logs'::text, 'moderation_actions'::text])))
);


--
-- Name: guild_members; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.guild_members (
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    display_name text,
    avatar text,
    roles jsonb DEFAULT '[]'::jsonb,
    joined_at timestamp with time zone,
    account_created timestamp with time zone,
    is_bot boolean DEFAULT false,
    last_seen_at timestamp with time zone DEFAULT now(),
    left_at timestamp with time zone
);


--
-- Name: guild_snapshots; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.guild_snapshots (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id text NOT NULL,
    label text,
    schema_version integer DEFAULT 1 NOT NULL,
    created_by text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    payload jsonb NOT NULL
);


--
-- Name: guilds; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.guilds (
    guild_id character varying(20) NOT NULL,
    name character varying(200) NOT NULL,
    icon character varying(200),
    member_count integer DEFAULT 0,
    registered_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: hourly_activity; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.hourly_activity (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    day date NOT NULL,
    hour smallint NOT NULL,
    messages bigint DEFAULT 0 NOT NULL,
    infractions integer DEFAULT 0 NOT NULL,
    CONSTRAINT hourly_activity_hour_check CHECK (((hour >= 0) AND (hour <= 23)))
);


--
-- Name: ia_config; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ia_config (
    guild_id character varying(20) NOT NULL,
    text_enabled boolean DEFAULT true NOT NULL,
    text_threshold double precision DEFAULT 0.5 NOT NULL,
    vision_enabled boolean DEFAULT true NOT NULL,
    vision_threshold double precision DEFAULT 0.5 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    context_dampening double precision DEFAULT 0.65 NOT NULL,
    context_format text DEFAULT 'natural'::text NOT NULL,
    context_max_messages integer DEFAULT 3 NOT NULL,
    context_max_chars integer DEFAULT 200 NOT NULL
);


--
-- Name: infractions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
)
PARTITION BY RANGE (created_at);


--
-- Name: infractions_2026_04; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2026_04 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_2026_05; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2026_05 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_2026_06; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2026_06 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_2026_07; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2026_07 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_2026_08; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2026_08 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_2026_09; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2026_09 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_2026_10; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2026_10 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_2026_11; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2026_11 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_2026_12; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2026_12 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_2027_01; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2027_01 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_2027_02; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2027_02 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_2027_03; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_2027_03 (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_default; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions_default (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    message_id character varying(20) NOT NULL,
    content text NOT NULL,
    flags jsonb NOT NULL,
    score double precision NOT NULL,
    action text NOT NULL,
    reason text NOT NULL,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: invitation_codes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.invitation_codes (
    code text NOT NULL,
    guild_id text NOT NULL,
    role text NOT NULL,
    created_by text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone,
    used_at timestamp with time zone,
    used_by_discord_id text,
    notes text,
    CONSTRAINT invitation_codes_role_check CHECK ((role = ANY (ARRAY['viewer'::text, 'moderator'::text, 'admin'::text, 'owner'::text])))
);


--
-- Name: logs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
)
PARTITION BY RANGE ("timestamp");


--
-- Name: logs_2026_04; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2026_04 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_2026_05; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2026_05 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_2026_06; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2026_06 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_2026_07; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2026_07 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_2026_08; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2026_08 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_2026_09; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2026_09 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_2026_10; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2026_10 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_2026_11; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2026_11 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_2026_12; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2026_12 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_2027_01; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2027_01 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_2027_02; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2027_02 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_2027_03; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_2027_03 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: logs_default; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.logs_default (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    level character varying(10) DEFAULT 'info'::character varying NOT NULL,
    bot character varying(100) DEFAULT ''::character varying NOT NULL,
    server character varying(200) DEFAULT ''::character varying NOT NULL,
    message text NOT NULL,
    category character varying(20) DEFAULT 'discord'::character varying NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: manual_ip_bans; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.manual_ip_bans (
    ip text NOT NULL,
    banned_at timestamp with time zone DEFAULT now() NOT NULL,
    banned_by text,
    reason text,
    unbanned_at timestamp with time zone,
    unbanned_by text
);


--
-- Name: manual_watched_users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.manual_watched_users (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    reason text DEFAULT ''::text NOT NULL,
    added_by text DEFAULT 'desktop'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: moderation_actions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.moderation_actions (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    moderator_id character varying(20) NOT NULL,
    moderator_name text NOT NULL,
    target_id character varying(20) NOT NULL,
    target_name text NOT NULL,
    action_type text NOT NULL,
    reason text NOT NULL,
    gravity public.moderation_gravity,
    duration bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: moderation_evidence; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.moderation_evidence (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    action_id uuid NOT NULL,
    url text NOT NULL,
    description text,
    uploaded_by character varying(20) NOT NULL,
    uploaded_by_name text NOT NULL,
    uploaded_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: moderation_sursis; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.moderation_sursis (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id text NOT NULL,
    user_id text NOT NULL,
    username text DEFAULT ''::text NOT NULL,
    moderator_id text DEFAULT ''::text NOT NULL,
    moderator_name text DEFAULT ''::text NOT NULL,
    reason text DEFAULT ''::text NOT NULL,
    saved_roles jsonb DEFAULT '[]'::jsonb NOT NULL,
    channel_id text,
    status text DEFAULT 'en_sursis'::text NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_levels; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_levels (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text DEFAULT ''::text NOT NULL,
    xp bigint DEFAULT 0 NOT NULL,
    level integer DEFAULT 0 NOT NULL,
    last_xp_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    streak_current integer DEFAULT 0 NOT NULL,
    streak_best integer DEFAULT 0 NOT NULL,
    streak_last_day integer DEFAULT 0 NOT NULL,
    streak_last_year integer DEFAULT 0 NOT NULL,
    xp_text bigint DEFAULT 0 NOT NULL,
    xp_voice bigint DEFAULT 0 NOT NULL,
    level_text integer DEFAULT 0 NOT NULL,
    level_voice integer DEFAULT 0 NOT NULL
);


--
-- Name: mv_level_leaderboard; Type: MATERIALIZED VIEW; Schema: public; Owner: -
--

CREATE MATERIALIZED VIEW public.mv_level_leaderboard AS
 SELECT id,
    guild_id,
    user_id,
    username,
    xp,
    level,
    xp_text,
    level_text,
    xp_voice,
    level_voice,
    row_number() OVER (PARTITION BY guild_id ORDER BY xp DESC) AS rank,
    last_xp_at,
    created_at,
    updated_at
   FROM public.user_levels
  WITH NO DATA;


--
-- Name: pending_mod_actions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.pending_mod_actions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    moderator_id character varying(20) NOT NULL,
    moderator_name text NOT NULL,
    target_id character varying(20) NOT NULL,
    target_name text NOT NULL,
    action_type text NOT NULL,
    reason text DEFAULT ''::text NOT NULL,
    gravity text,
    duration bigint,
    status text DEFAULT 'pending'::text NOT NULL,
    reviewed_by text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: pending_role_grants; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.pending_role_grants (
    guild_id text NOT NULL,
    user_id text NOT NULL,
    role_ids jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: rbac_component_min_role; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.rbac_component_min_role (
    guild_id character varying(20) NOT NULL,
    component_key character varying(100) NOT NULL,
    min_role character varying(20) NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_by character varying(20),
    CONSTRAINT chk_rbac_min_role CHECK (((min_role)::text = ANY ((ARRAY['viewer'::character varying, 'moderator'::character varying, 'admin'::character varying, 'owner'::character varying])::text[])))
);


--
-- Name: rbac_component_visibility; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.rbac_component_visibility (
    guild_id text NOT NULL,
    component_key text NOT NULL,
    role text NOT NULL,
    visible boolean NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_by text,
    CONSTRAINT rbac_component_visibility_role_check CHECK ((role = ANY (ARRAY['viewer'::text, 'moderator'::text, 'admin'::text, 'owner'::text])))
);


--
-- Name: review_queue; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.review_queue (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    action_id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    added_by character varying(20) NOT NULL,
    added_by_name text NOT NULL,
    reason text,
    status text DEFAULT 'pending'::text NOT NULL,
    reviewer_id character varying(20),
    reviewer_name text,
    reviewer_notes text,
    added_at timestamp with time zone DEFAULT now() NOT NULL,
    resolved_at timestamp with time zone,
    CONSTRAINT review_queue_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'approved'::text, 'rejected'::text, 'changed'::text])))
);


--
-- Name: role_panel_entries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.role_panel_entries (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    panel_id uuid NOT NULL,
    role_id character varying(20) NOT NULL,
    role_name text DEFAULT ''::text NOT NULL,
    emoji text,
    label text DEFAULT ''::text NOT NULL,
    style text DEFAULT 'primary'::text NOT NULL,
    "position" integer DEFAULT 0 NOT NULL
);


--
-- Name: role_panels; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.role_panels (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    message_id character varying(20),
    title text NOT NULL,
    description text DEFAULT ''::text NOT NULL,
    mode text DEFAULT 'button'::text NOT NULL,
    max_roles integer,
    enabled boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: rules; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.rules (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    flag_type text NOT NULL,
    weight double precision DEFAULT 1.0 NOT NULL,
    threshold_warn double precision DEFAULT 2.0 NOT NULL,
    threshold_delete double precision DEFAULT 4.0 NOT NULL,
    threshold_mute double precision DEFAULT 6.0 NOT NULL,
    threshold_ban double precision DEFAULT 9.0 NOT NULL,
    enabled boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: sanction_reminders; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sanction_reminders (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    moderator_id character varying(20) NOT NULL,
    moderator_name text NOT NULL,
    target_id character varying(20) NOT NULL,
    target_name text NOT NULL,
    action_type text NOT NULL,
    reason text NOT NULL,
    action_id uuid NOT NULL,
    remind_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    unban_status text DEFAULT 'pending'::text NOT NULL
);


--
-- Name: scheduled_announcement_runs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.scheduled_announcement_runs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    announcement_id uuid NOT NULL,
    guild_id text NOT NULL,
    ran_at timestamp with time zone DEFAULT now() NOT NULL,
    channels_posted jsonb DEFAULT '[]'::jsonb NOT NULL,
    status text NOT NULL,
    error text,
    CONSTRAINT scheduled_announcement_runs_status_check CHECK ((status = ANY (ARRAY['success'::text, 'partial'::text, 'error'::text, 'pending'::text])))
);


--
-- Name: scheduled_announcements; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.scheduled_announcements (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id text NOT NULL,
    name text NOT NULL,
    enabled boolean DEFAULT true NOT NULL,
    recurrence_type text NOT NULL,
    recurrence_hour smallint NOT NULL,
    recurrence_minute smallint DEFAULT 0 NOT NULL,
    recurrence_day_of_week smallint,
    recurrence_day_of_month smallint,
    scheduled_at timestamp with time zone,
    start_date timestamp with time zone DEFAULT now() NOT NULL,
    end_date timestamp with time zone,
    content_type text NOT NULL,
    content_text text DEFAULT ''::text NOT NULL,
    embed_title text,
    embed_color integer,
    embed_image_url text,
    embed_thumbnail_url text,
    mention_everyone boolean DEFAULT false NOT NULL,
    mention_here boolean DEFAULT false NOT NULL,
    mention_role_ids jsonb DEFAULT '[]'::jsonb NOT NULL,
    channel_ids jsonb NOT NULL,
    created_by text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    last_run_at timestamp with time zone,
    next_run_at timestamp with time zone NOT NULL,
    buttons jsonb DEFAULT '[]'::jsonb NOT NULL,
    auto_reactions jsonb DEFAULT '[]'::jsonb NOT NULL,
    CONSTRAINT recurrence_consistency CHECK ((((recurrence_type = 'once'::text) AND (scheduled_at IS NOT NULL)) OR (recurrence_type = 'daily'::text) OR ((recurrence_type = 'weekly'::text) AND (recurrence_day_of_week IS NOT NULL)) OR ((recurrence_type = 'monthly'::text) AND (recurrence_day_of_month IS NOT NULL)))),
    CONSTRAINT scheduled_announcements_content_type_check CHECK ((content_type = ANY (ARRAY['text'::text, 'embed'::text]))),
    CONSTRAINT scheduled_announcements_recurrence_day_of_month_check CHECK (((recurrence_day_of_month >= 1) AND (recurrence_day_of_month <= 31))),
    CONSTRAINT scheduled_announcements_recurrence_day_of_week_check CHECK (((recurrence_day_of_week >= 0) AND (recurrence_day_of_week <= 6))),
    CONSTRAINT scheduled_announcements_recurrence_hour_check CHECK (((recurrence_hour >= 0) AND (recurrence_hour <= 23))),
    CONSTRAINT scheduled_announcements_recurrence_minute_check CHECK (((recurrence_minute >= 0) AND (recurrence_minute <= 59))),
    CONSTRAINT scheduled_announcements_recurrence_type_check CHECK ((recurrence_type = ANY (ARRAY['once'::text, 'daily'::text, 'weekly'::text, 'monthly'::text])))
);


--
-- Name: security_events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.security_events (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    severity text NOT NULL,
    description text NOT NULL,
    user_ids jsonb DEFAULT '[]'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: security_lockdown_active; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.security_lockdown_active (
    guild_id text NOT NULL,
    saved_states jsonb NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: security_quarantine_pending; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.security_quarantine_pending (
    guild_id text NOT NULL,
    user_id text NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: security_slowmode_active; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.security_slowmode_active (
    guild_id text NOT NULL,
    previous_states jsonb NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    imposed_rate integer DEFAULT 0 NOT NULL
);


--
-- Name: server_events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.server_events (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    actor text,
    actor_name text,
    action text NOT NULL,
    target text,
    severity text DEFAULT 'info'::text NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL,
    CONSTRAINT server_events_severity_check CHECK ((severity = ANY (ARRAY['info'::text, 'warn'::text, 'critical'::text])))
);


--
-- Name: sponsorships; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sponsorships (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    sponsor_id text NOT NULL,
    sponsored_id text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: strike_config; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.strike_config (
    guild_id character varying(20) NOT NULL,
    window_secs bigint DEFAULT 3600 NOT NULL,
    thresholds jsonb DEFAULT '[]'::jsonb NOT NULL,
    enabled boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: successful_logins; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.successful_logins (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    logged_at timestamp with time zone DEFAULT now() NOT NULL,
    discord_user_id text NOT NULL,
    username text,
    client_ip text,
    user_agent text
);


--
-- Name: temp_roles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.temp_roles (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    role_id character varying(20) NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: ticket_assignments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ticket_assignments (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    ticket_id uuid NOT NULL,
    assigned_to character varying(20) NOT NULL,
    assigned_by text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: ticket_messages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ticket_messages (
    id uuid NOT NULL,
    ticket_id uuid NOT NULL,
    author_name text NOT NULL,
    author_role text DEFAULT 'user'::text NOT NULL,
    content text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: tickets; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.tickets (
    id uuid NOT NULL,
    title text NOT NULL,
    status text DEFAULT 'open'::text NOT NULL,
    priority text DEFAULT 'medium'::text NOT NULL,
    author_id character varying(20) NOT NULL,
    author_name text NOT NULL,
    assigned_to character varying(20),
    server text NOT NULL,
    category text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    ticket_type text DEFAULT 'autre'::text NOT NULL,
    channel_id character varying(20),
    voice_channel_id character varying(20),
    invited_user_id character varying(20),
    first_response_at timestamp with time zone,
    resolved_at timestamp with time zone,
    satisfaction_rating integer,
    escalated_at timestamp with time zone,
    sla_warned_at timestamp with time zone,
    guild_id text
);


--
-- Name: user_activity_log; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
)
PARTITION BY RANGE (created_at);


--
-- Name: user_activity_log_2026_04; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2026_04 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_2026_05; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2026_05 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_2026_06; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2026_06 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_2026_07; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2026_07 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_2026_08; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2026_08 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_2026_09; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2026_09 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_2026_10; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2026_10 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_2026_11; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2026_11 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_2026_12; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2026_12 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_2027_01; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2027_01 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_2027_02; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2027_02 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_2027_03; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_2027_03 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_activity_log_default; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_activity_log_default (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    event_type text NOT NULL,
    channel_id character varying(20),
    channel_name text,
    content text,
    metadata jsonb DEFAULT '{}'::jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_cache; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_cache (
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    avatar_url text,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_levels_monthly_snapshot; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_levels_monthly_snapshot (
    guild_id text NOT NULL,
    user_id text NOT NULL,
    period_ym text NOT NULL,
    xp_text bigint DEFAULT 0 NOT NULL,
    xp_voice bigint DEFAULT 0 NOT NULL,
    partial boolean DEFAULT false NOT NULL,
    captured_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_notes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_notes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    author_id character varying(20) NOT NULL,
    author_name text NOT NULL,
    content text NOT NULL,
    category text DEFAULT 'general'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_stats; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_stats (
    id uuid NOT NULL,
    guild_id character varying NOT NULL,
    user_id character varying NOT NULL,
    username character varying DEFAULT ''::character varying NOT NULL,
    message_count bigint DEFAULT 0 NOT NULL,
    voice_seconds bigint DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_strikes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_strikes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    reason text NOT NULL,
    source text NOT NULL,
    infraction_id uuid,
    expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: voice_channel_bans; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.voice_channel_bans (
    id uuid NOT NULL,
    voice_channel_id uuid,
    user_id character varying(20) NOT NULL,
    user_name text NOT NULL,
    banned_by character varying(20) NOT NULL,
    reason text,
    expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    guild_id text,
    owner_id text
);


--
-- Name: voice_channel_co_admins; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.voice_channel_co_admins (
    id uuid NOT NULL,
    voice_channel_id uuid NOT NULL,
    user_id character varying(20) NOT NULL,
    user_name text NOT NULL,
    granted_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: voice_channel_invite_links; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.voice_channel_invite_links (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    voice_channel_id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    channel_id character varying(20) NOT NULL,
    created_by text NOT NULL,
    created_by_name text NOT NULL,
    code text NOT NULL,
    max_uses integer,
    current_uses integer DEFAULT 0 NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    revoked boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: voice_channel_presets; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.voice_channel_presets (
    guild_id text NOT NULL,
    owner_id text NOT NULL,
    channel_name text,
    member_limit integer,
    visibility text DEFAULT 'visible'::text NOT NULL,
    locked boolean DEFAULT false NOT NULL,
    queue_enabled boolean DEFAULT false NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: voice_channel_themes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.voice_channel_themes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    name text NOT NULL,
    emoji text,
    channel_name_template text DEFAULT '{user}'::text NOT NULL,
    member_limit integer,
    visibility text DEFAULT 'visible'::text NOT NULL,
    locked boolean DEFAULT false NOT NULL,
    queue_enabled boolean DEFAULT false NOT NULL,
    bitrate integer,
    slowmode_secs integer,
    is_default boolean DEFAULT false NOT NULL,
    sort_order integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    stage_enabled boolean DEFAULT false NOT NULL
);


--
-- Name: voice_channel_whitelists; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.voice_channel_whitelists (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    owner_id character varying(20) NOT NULL,
    target_id character varying(20) NOT NULL,
    target_name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: voice_channels; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.voice_channels (
    id uuid NOT NULL,
    guild_id character varying(20) NOT NULL,
    owner_id character varying(20) NOT NULL,
    owner_name text NOT NULL,
    channel_id character varying(20) NOT NULL,
    text_channel_id character varying(20),
    members_channel_id character varying(20),
    queue_channel_id character varying(20),
    category_id character varying(20),
    channel_name text NOT NULL,
    kind public.voice_channel_kind DEFAULT 'public'::public.voice_channel_kind NOT NULL,
    visibility text DEFAULT 'visible'::text NOT NULL,
    queue_enabled boolean DEFAULT false NOT NULL,
    locked boolean DEFAULT false NOT NULL,
    member_limit integer,
    status text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    channel_status character varying(10) DEFAULT 'open'::character varying NOT NULL,
    closed_at timestamp with time zone,
    stage_enabled boolean DEFAULT false NOT NULL
);


--
-- Name: voice_sessions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.voice_sessions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    guild_id character varying(20) NOT NULL,
    user_id character varying(20) NOT NULL,
    username text NOT NULL,
    channel_id character varying(20) NOT NULL,
    channel_name text DEFAULT ''::text NOT NULL,
    duration_secs bigint DEFAULT 0 NOT NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    ended_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: web_oauth_sessions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.web_oauth_sessions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    discord_user_id text NOT NULL,
    username text DEFAULT ''::text NOT NULL,
    global_name text,
    avatar text,
    access_token text NOT NULL,
    refresh_token text NOT NULL,
    access_expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: welcome_config; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.welcome_config (
    guild_id character varying(20) NOT NULL,
    welcome_enabled boolean DEFAULT true NOT NULL,
    welcome_channel_id character varying(20),
    welcome_message text DEFAULT 'Bienvenue {user} sur **{server}** ! Tu es le **{count}e** membre.'::text NOT NULL,
    welcome_embed_color text DEFAULT '3498db'::text NOT NULL,
    welcome_dm_enabled boolean DEFAULT false NOT NULL,
    welcome_dm_message text DEFAULT 'Bienvenue sur **{server}** ! N''oublie pas de lire les regles.'::text NOT NULL,
    leave_enabled boolean DEFAULT true NOT NULL,
    leave_channel_id character varying(20),
    leave_message text DEFAULT '{user} nous a quittes. Nous sommes maintenant **{count}** membres.'::text NOT NULL,
    rules_enabled boolean DEFAULT false NOT NULL,
    rules_channel_id character varying(20),
    rules_message text DEFAULT 'Lis les regles et clique sur le bouton pour acceder au serveur.'::text NOT NULL,
    rules_role_id character varying(20),
    rules_button_label text DEFAULT 'J''accepte les regles'::text NOT NULL,
    counter_enabled boolean DEFAULT false NOT NULL,
    counter_channel_id character varying(20),
    counter_format text DEFAULT 'Membres : {count}'::text NOT NULL,
    anniversary_enabled boolean DEFAULT false NOT NULL,
    anniversary_channel_id character varying(20),
    anniversary_message text DEFAULT 'Felicitations {user}, ca fait **{years} an(s)** que tu es sur **{server}** !'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    rejoin_message text DEFAULT 'Content de te revoir {user} ! Tu nous avais manque.'::text NOT NULL,
    age_check_enabled boolean DEFAULT false NOT NULL,
    age_minimum integer DEFAULT 20 NOT NULL,
    unverified_role_id text,
    age_modal_question text DEFAULT 'Quel age as-tu ? (en chiffres)'::text NOT NULL,
    age_ban_message text DEFAULT 'Tu dois avoir au moins {min} ans pour rejoindre ce serveur. Tu pourras revenir dans {annees} an(s).'::text NOT NULL
);


--
-- Name: audit_logs_2026_04; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2026_04 FOR VALUES FROM ('2026-04-01 00:00:00+00') TO ('2026-05-01 00:00:00+00');


--
-- Name: audit_logs_2026_05; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2026_05 FOR VALUES FROM ('2026-05-01 00:00:00+00') TO ('2026-06-01 00:00:00+00');


--
-- Name: audit_logs_2026_06; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2026_06 FOR VALUES FROM ('2026-06-01 00:00:00+00') TO ('2026-07-01 00:00:00+00');


--
-- Name: audit_logs_2026_07; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2026_07 FOR VALUES FROM ('2026-07-01 00:00:00+00') TO ('2026-08-01 00:00:00+00');


--
-- Name: audit_logs_2026_08; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2026_08 FOR VALUES FROM ('2026-08-01 00:00:00+00') TO ('2026-09-01 00:00:00+00');


--
-- Name: audit_logs_2026_09; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2026_09 FOR VALUES FROM ('2026-09-01 00:00:00+00') TO ('2026-10-01 00:00:00+00');


--
-- Name: audit_logs_2026_10; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2026_10 FOR VALUES FROM ('2026-10-01 00:00:00+00') TO ('2026-11-01 00:00:00+00');


--
-- Name: audit_logs_2026_11; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2026_11 FOR VALUES FROM ('2026-11-01 00:00:00+00') TO ('2026-12-01 00:00:00+00');


--
-- Name: audit_logs_2026_12; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2026_12 FOR VALUES FROM ('2026-12-01 00:00:00+00') TO ('2027-01-01 00:00:00+00');


--
-- Name: audit_logs_2027_01; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2027_01 FOR VALUES FROM ('2027-01-01 00:00:00+00') TO ('2027-02-01 00:00:00+00');


--
-- Name: audit_logs_2027_02; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2027_02 FOR VALUES FROM ('2027-02-01 00:00:00+00') TO ('2027-03-01 00:00:00+00');


--
-- Name: audit_logs_2027_03; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_2027_03 FOR VALUES FROM ('2027-03-01 00:00:00+00') TO ('2027-04-01 00:00:00+00');


--
-- Name: audit_logs_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ATTACH PARTITION public.audit_logs_default DEFAULT;


--
-- Name: infractions_2026_04; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2026_04 FOR VALUES FROM ('2026-04-01 00:00:00+00') TO ('2026-05-01 00:00:00+00');


--
-- Name: infractions_2026_05; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2026_05 FOR VALUES FROM ('2026-05-01 00:00:00+00') TO ('2026-06-01 00:00:00+00');


--
-- Name: infractions_2026_06; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2026_06 FOR VALUES FROM ('2026-06-01 00:00:00+00') TO ('2026-07-01 00:00:00+00');


--
-- Name: infractions_2026_07; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2026_07 FOR VALUES FROM ('2026-07-01 00:00:00+00') TO ('2026-08-01 00:00:00+00');


--
-- Name: infractions_2026_08; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2026_08 FOR VALUES FROM ('2026-08-01 00:00:00+00') TO ('2026-09-01 00:00:00+00');


--
-- Name: infractions_2026_09; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2026_09 FOR VALUES FROM ('2026-09-01 00:00:00+00') TO ('2026-10-01 00:00:00+00');


--
-- Name: infractions_2026_10; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2026_10 FOR VALUES FROM ('2026-10-01 00:00:00+00') TO ('2026-11-01 00:00:00+00');


--
-- Name: infractions_2026_11; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2026_11 FOR VALUES FROM ('2026-11-01 00:00:00+00') TO ('2026-12-01 00:00:00+00');


--
-- Name: infractions_2026_12; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2026_12 FOR VALUES FROM ('2026-12-01 00:00:00+00') TO ('2027-01-01 00:00:00+00');


--
-- Name: infractions_2027_01; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2027_01 FOR VALUES FROM ('2027-01-01 00:00:00+00') TO ('2027-02-01 00:00:00+00');


--
-- Name: infractions_2027_02; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2027_02 FOR VALUES FROM ('2027-02-01 00:00:00+00') TO ('2027-03-01 00:00:00+00');


--
-- Name: infractions_2027_03; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_2027_03 FOR VALUES FROM ('2027-03-01 00:00:00+00') TO ('2027-04-01 00:00:00+00');


--
-- Name: infractions_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ATTACH PARTITION public.infractions_default DEFAULT;


--
-- Name: logs_2026_04; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2026_04 FOR VALUES FROM ('2026-04-01 00:00:00+00') TO ('2026-05-01 00:00:00+00');


--
-- Name: logs_2026_05; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2026_05 FOR VALUES FROM ('2026-05-01 00:00:00+00') TO ('2026-06-01 00:00:00+00');


--
-- Name: logs_2026_06; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2026_06 FOR VALUES FROM ('2026-06-01 00:00:00+00') TO ('2026-07-01 00:00:00+00');


--
-- Name: logs_2026_07; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2026_07 FOR VALUES FROM ('2026-07-01 00:00:00+00') TO ('2026-08-01 00:00:00+00');


--
-- Name: logs_2026_08; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2026_08 FOR VALUES FROM ('2026-08-01 00:00:00+00') TO ('2026-09-01 00:00:00+00');


--
-- Name: logs_2026_09; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2026_09 FOR VALUES FROM ('2026-09-01 00:00:00+00') TO ('2026-10-01 00:00:00+00');


--
-- Name: logs_2026_10; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2026_10 FOR VALUES FROM ('2026-10-01 00:00:00+00') TO ('2026-11-01 00:00:00+00');


--
-- Name: logs_2026_11; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2026_11 FOR VALUES FROM ('2026-11-01 00:00:00+00') TO ('2026-12-01 00:00:00+00');


--
-- Name: logs_2026_12; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2026_12 FOR VALUES FROM ('2026-12-01 00:00:00+00') TO ('2027-01-01 00:00:00+00');


--
-- Name: logs_2027_01; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2027_01 FOR VALUES FROM ('2027-01-01 00:00:00+00') TO ('2027-02-01 00:00:00+00');


--
-- Name: logs_2027_02; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2027_02 FOR VALUES FROM ('2027-02-01 00:00:00+00') TO ('2027-03-01 00:00:00+00');


--
-- Name: logs_2027_03; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_2027_03 FOR VALUES FROM ('2027-03-01 00:00:00+00') TO ('2027-04-01 00:00:00+00');


--
-- Name: logs_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs ATTACH PARTITION public.logs_default DEFAULT;


--
-- Name: user_activity_log_2026_04; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2026_04 FOR VALUES FROM ('2026-04-01 00:00:00+00') TO ('2026-05-01 00:00:00+00');


--
-- Name: user_activity_log_2026_05; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2026_05 FOR VALUES FROM ('2026-05-01 00:00:00+00') TO ('2026-06-01 00:00:00+00');


--
-- Name: user_activity_log_2026_06; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2026_06 FOR VALUES FROM ('2026-06-01 00:00:00+00') TO ('2026-07-01 00:00:00+00');


--
-- Name: user_activity_log_2026_07; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2026_07 FOR VALUES FROM ('2026-07-01 00:00:00+00') TO ('2026-08-01 00:00:00+00');


--
-- Name: user_activity_log_2026_08; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2026_08 FOR VALUES FROM ('2026-08-01 00:00:00+00') TO ('2026-09-01 00:00:00+00');


--
-- Name: user_activity_log_2026_09; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2026_09 FOR VALUES FROM ('2026-09-01 00:00:00+00') TO ('2026-10-01 00:00:00+00');


--
-- Name: user_activity_log_2026_10; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2026_10 FOR VALUES FROM ('2026-10-01 00:00:00+00') TO ('2026-11-01 00:00:00+00');


--
-- Name: user_activity_log_2026_11; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2026_11 FOR VALUES FROM ('2026-11-01 00:00:00+00') TO ('2026-12-01 00:00:00+00');


--
-- Name: user_activity_log_2026_12; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2026_12 FOR VALUES FROM ('2026-12-01 00:00:00+00') TO ('2027-01-01 00:00:00+00');


--
-- Name: user_activity_log_2027_01; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2027_01 FOR VALUES FROM ('2027-01-01 00:00:00+00') TO ('2027-02-01 00:00:00+00');


--
-- Name: user_activity_log_2027_02; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2027_02 FOR VALUES FROM ('2027-02-01 00:00:00+00') TO ('2027-03-01 00:00:00+00');


--
-- Name: user_activity_log_2027_03; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_2027_03 FOR VALUES FROM ('2027-03-01 00:00:00+00') TO ('2027-04-01 00:00:00+00');


--
-- Name: user_activity_log_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log ATTACH PARTITION public.user_activity_log_default DEFAULT;


--
-- Name: admin_rotation_history admin_rotation_history_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.admin_rotation_history
    ADD CONSTRAINT admin_rotation_history_pkey PRIMARY KEY (id);


--
-- Name: admin_rotation admin_rotation_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.admin_rotation
    ADD CONSTRAINT admin_rotation_pkey PRIMARY KEY (guild_id);


--
-- Name: age_verification_bans age_verification_bans_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.age_verification_bans
    ADD CONSTRAINT age_verification_bans_pkey PRIMARY KEY (id);


--
-- Name: ai_dataset_messages ai_dataset_messages_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ai_dataset_messages
    ADD CONSTRAINT ai_dataset_messages_pkey PRIMARY KEY (id);


--
-- Name: ai_jobs ai_jobs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ai_jobs
    ADD CONSTRAINT ai_jobs_pkey PRIMARY KEY (id);


--
-- Name: alert_rules alert_rules_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.alert_rules
    ADD CONSTRAINT alert_rules_pkey PRIMARY KEY (id);


--
-- Name: analytics_daily_baseline analytics_daily_baseline_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.analytics_daily_baseline
    ADD CONSTRAINT analytics_daily_baseline_pkey PRIMARY KEY (guild_id, day);


--
-- Name: announcement_button_interactions announcement_button_interactions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.announcement_button_interactions
    ADD CONSTRAINT announcement_button_interactions_pkey PRIMARY KEY (id);


--
-- Name: api_user_guilds api_user_guilds_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.api_user_guilds
    ADD CONSTRAINT api_user_guilds_pkey PRIMARY KEY (discord_user_id, guild_id);


--
-- Name: api_users api_users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.api_users
    ADD CONSTRAINT api_users_pkey PRIMARY KEY (discord_user_id);


--
-- Name: audit_logs audit_logs_pkey1; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_pkey1 PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2026_04 audit_logs_2026_04_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2026_04
    ADD CONSTRAINT audit_logs_2026_04_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2026_05 audit_logs_2026_05_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2026_05
    ADD CONSTRAINT audit_logs_2026_05_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2026_06 audit_logs_2026_06_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2026_06
    ADD CONSTRAINT audit_logs_2026_06_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2026_07 audit_logs_2026_07_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2026_07
    ADD CONSTRAINT audit_logs_2026_07_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2026_08 audit_logs_2026_08_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2026_08
    ADD CONSTRAINT audit_logs_2026_08_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2026_09 audit_logs_2026_09_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2026_09
    ADD CONSTRAINT audit_logs_2026_09_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2026_10 audit_logs_2026_10_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2026_10
    ADD CONSTRAINT audit_logs_2026_10_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2026_11 audit_logs_2026_11_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2026_11
    ADD CONSTRAINT audit_logs_2026_11_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2026_12 audit_logs_2026_12_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2026_12
    ADD CONSTRAINT audit_logs_2026_12_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2027_01 audit_logs_2027_01_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2027_01
    ADD CONSTRAINT audit_logs_2027_01_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2027_02 audit_logs_2027_02_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2027_02
    ADD CONSTRAINT audit_logs_2027_02_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_2027_03 audit_logs_2027_03_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_2027_03
    ADD CONSTRAINT audit_logs_2027_03_pkey PRIMARY KEY (id, created_at);


--
-- Name: audit_logs_default audit_logs_default_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs_default
    ADD CONSTRAINT audit_logs_default_pkey PRIMARY KEY (id, created_at);


--
-- Name: auto_roles auto_roles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.auto_roles
    ADD CONSTRAINT auto_roles_pkey PRIMARY KEY (id);


--
-- Name: automod_adaptive_slowmode automod_adaptive_slowmode_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.automod_adaptive_slowmode
    ADD CONSTRAINT automod_adaptive_slowmode_pkey PRIMARY KEY (channel_id);


--
-- Name: automod_discussion_channels automod_discussion_channels_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.automod_discussion_channels
    ADD CONSTRAINT automod_discussion_channels_pkey PRIMARY KEY (id);


--
-- Name: automod_discussion_channels automod_discussion_channels_review_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.automod_discussion_channels
    ADD CONSTRAINT automod_discussion_channels_review_id_key UNIQUE (review_id);


--
-- Name: automod_discussion_messages automod_discussion_messages_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.automod_discussion_messages
    ADD CONSTRAINT automod_discussion_messages_pkey PRIMARY KEY (id);


--
-- Name: automod_discussion_messages automod_discussion_messages_review_id_discord_message_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.automod_discussion_messages
    ADD CONSTRAINT automod_discussion_messages_review_id_discord_message_id_key UNIQUE (review_id, discord_message_id);


--
-- Name: automod_review_votes automod_review_votes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.automod_review_votes
    ADD CONSTRAINT automod_review_votes_pkey PRIMARY KEY (id);


--
-- Name: automod_review_votes automod_review_votes_review_id_voter_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.automod_review_votes
    ADD CONSTRAINT automod_review_votes_review_id_voter_id_key UNIQUE (review_id, voter_id);


--
-- Name: automod_reviews automod_reviews_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.automod_reviews
    ADD CONSTRAINT automod_reviews_pkey PRIMARY KEY (id);


--
-- Name: bot_definitions bot_definitions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bot_definitions
    ADD CONSTRAINT bot_definitions_pkey PRIMARY KEY (bot_name);


--
-- Name: bot_guild_config bot_guild_config_guild_id_bot_name_config_key_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bot_guild_config
    ADD CONSTRAINT bot_guild_config_guild_id_bot_name_config_key_key UNIQUE (guild_id, bot_name, config_key);


--
-- Name: bot_guild_config bot_guild_config_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bot_guild_config
    ADD CONSTRAINT bot_guild_config_pkey PRIMARY KEY (id);


--
-- Name: bump_events bump_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bump_events
    ADD CONSTRAINT bump_events_pkey PRIMARY KEY (id);


--
-- Name: bump_guild_state bump_guild_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bump_guild_state
    ADD CONSTRAINT bump_guild_state_pkey PRIMARY KEY (guild_id, provider);


--
-- Name: confession_config confession_config_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.confession_config
    ADD CONSTRAINT confession_config_pkey PRIMARY KEY (guild_id);


--
-- Name: confession_counters confession_counters_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.confession_counters
    ADD CONSTRAINT confession_counters_pkey PRIMARY KEY (guild_id);


--
-- Name: confession_replies confession_replies_confession_id_public_number_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.confession_replies
    ADD CONSTRAINT confession_replies_confession_id_public_number_key UNIQUE (confession_id, public_number);


--
-- Name: confession_replies confession_replies_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.confession_replies
    ADD CONSTRAINT confession_replies_pkey PRIMARY KEY (id);


--
-- Name: confession_reports confession_reports_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.confession_reports
    ADD CONSTRAINT confession_reports_pkey PRIMARY KEY (id);


--
-- Name: confessions confessions_guild_id_public_number_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.confessions
    ADD CONSTRAINT confessions_guild_id_public_number_key UNIQUE (guild_id, public_number);


--
-- Name: confessions confessions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.confessions
    ADD CONSTRAINT confessions_pkey PRIMARY KEY (id);


--
-- Name: daily_activity daily_activity_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.daily_activity
    ADD CONSTRAINT daily_activity_pkey PRIMARY KEY (id);


--
-- Name: discord_action_messages discord_action_messages_guild_id_channel_id_message_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.discord_action_messages
    ADD CONSTRAINT discord_action_messages_guild_id_channel_id_message_id_key UNIQUE (guild_id, channel_id, message_id);


--
-- Name: discord_action_messages discord_action_messages_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.discord_action_messages
    ADD CONSTRAINT discord_action_messages_pkey PRIMARY KEY (action_id, kind);


--
-- Name: discord_audit_sync_state discord_audit_sync_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.discord_audit_sync_state
    ADD CONSTRAINT discord_audit_sync_state_pkey PRIMARY KEY (guild_id);


--
-- Name: discord_roles discord_roles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.discord_roles
    ADD CONSTRAINT discord_roles_pkey PRIMARY KEY (guild_id, id);


--
-- Name: export_jobs export_jobs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.export_jobs
    ADD CONSTRAINT export_jobs_pkey PRIMARY KEY (id);


--
-- Name: guild_members guild_members_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.guild_members
    ADD CONSTRAINT guild_members_pkey PRIMARY KEY (guild_id, user_id);


--
-- Name: guild_snapshots guild_snapshots_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.guild_snapshots
    ADD CONSTRAINT guild_snapshots_pkey PRIMARY KEY (id);


--
-- Name: guilds guilds_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.guilds
    ADD CONSTRAINT guilds_pkey PRIMARY KEY (guild_id);


--
-- Name: hourly_activity hourly_activity_guild_id_day_hour_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.hourly_activity
    ADD CONSTRAINT hourly_activity_guild_id_day_hour_key UNIQUE (guild_id, day, hour);


--
-- Name: hourly_activity hourly_activity_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.hourly_activity
    ADD CONSTRAINT hourly_activity_pkey PRIMARY KEY (id);


--
-- Name: ia_config ia_config_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ia_config
    ADD CONSTRAINT ia_config_pkey PRIMARY KEY (guild_id);


--
-- Name: infractions infractions_pkey1; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions
    ADD CONSTRAINT infractions_pkey1 PRIMARY KEY (id, created_at);


--
-- Name: infractions_2026_04 infractions_2026_04_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2026_04
    ADD CONSTRAINT infractions_2026_04_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_2026_05 infractions_2026_05_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2026_05
    ADD CONSTRAINT infractions_2026_05_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_2026_06 infractions_2026_06_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2026_06
    ADD CONSTRAINT infractions_2026_06_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_2026_07 infractions_2026_07_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2026_07
    ADD CONSTRAINT infractions_2026_07_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_2026_08 infractions_2026_08_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2026_08
    ADD CONSTRAINT infractions_2026_08_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_2026_09 infractions_2026_09_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2026_09
    ADD CONSTRAINT infractions_2026_09_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_2026_10 infractions_2026_10_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2026_10
    ADD CONSTRAINT infractions_2026_10_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_2026_11 infractions_2026_11_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2026_11
    ADD CONSTRAINT infractions_2026_11_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_2026_12 infractions_2026_12_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2026_12
    ADD CONSTRAINT infractions_2026_12_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_2027_01 infractions_2027_01_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2027_01
    ADD CONSTRAINT infractions_2027_01_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_2027_02 infractions_2027_02_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2027_02
    ADD CONSTRAINT infractions_2027_02_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_2027_03 infractions_2027_03_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_2027_03
    ADD CONSTRAINT infractions_2027_03_pkey PRIMARY KEY (id, created_at);


--
-- Name: infractions_default infractions_default_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions_default
    ADD CONSTRAINT infractions_default_pkey PRIMARY KEY (id, created_at);


--
-- Name: invitation_codes invitation_codes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.invitation_codes
    ADD CONSTRAINT invitation_codes_pkey PRIMARY KEY (code);


--
-- Name: logs logs_pkey1; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs
    ADD CONSTRAINT logs_pkey1 PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2026_04 logs_2026_04_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2026_04
    ADD CONSTRAINT logs_2026_04_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2026_05 logs_2026_05_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2026_05
    ADD CONSTRAINT logs_2026_05_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2026_06 logs_2026_06_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2026_06
    ADD CONSTRAINT logs_2026_06_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2026_07 logs_2026_07_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2026_07
    ADD CONSTRAINT logs_2026_07_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2026_08 logs_2026_08_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2026_08
    ADD CONSTRAINT logs_2026_08_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2026_09 logs_2026_09_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2026_09
    ADD CONSTRAINT logs_2026_09_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2026_10 logs_2026_10_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2026_10
    ADD CONSTRAINT logs_2026_10_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2026_11 logs_2026_11_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2026_11
    ADD CONSTRAINT logs_2026_11_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2026_12 logs_2026_12_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2026_12
    ADD CONSTRAINT logs_2026_12_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2027_01 logs_2027_01_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2027_01
    ADD CONSTRAINT logs_2027_01_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2027_02 logs_2027_02_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2027_02
    ADD CONSTRAINT logs_2027_02_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_2027_03 logs_2027_03_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_2027_03
    ADD CONSTRAINT logs_2027_03_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: logs_default logs_default_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.logs_default
    ADD CONSTRAINT logs_default_pkey PRIMARY KEY (id, "timestamp");


--
-- Name: manual_ip_bans manual_ip_bans_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.manual_ip_bans
    ADD CONSTRAINT manual_ip_bans_pkey PRIMARY KEY (ip);


--
-- Name: manual_watched_users manual_watched_users_guild_id_user_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.manual_watched_users
    ADD CONSTRAINT manual_watched_users_guild_id_user_id_key UNIQUE (guild_id, user_id);


--
-- Name: manual_watched_users manual_watched_users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.manual_watched_users
    ADD CONSTRAINT manual_watched_users_pkey PRIMARY KEY (id);


--
-- Name: moderation_actions moderation_actions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.moderation_actions
    ADD CONSTRAINT moderation_actions_pkey PRIMARY KEY (id);


--
-- Name: moderation_evidence moderation_evidence_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.moderation_evidence
    ADD CONSTRAINT moderation_evidence_pkey PRIMARY KEY (id);


--
-- Name: moderation_sursis moderation_sursis_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.moderation_sursis
    ADD CONSTRAINT moderation_sursis_pkey PRIMARY KEY (id);


--
-- Name: pending_mod_actions pending_mod_actions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.pending_mod_actions
    ADD CONSTRAINT pending_mod_actions_pkey PRIMARY KEY (id);


--
-- Name: pending_role_grants pending_role_grants_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.pending_role_grants
    ADD CONSTRAINT pending_role_grants_pkey PRIMARY KEY (guild_id, user_id);


--
-- Name: rbac_component_min_role rbac_component_min_role_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rbac_component_min_role
    ADD CONSTRAINT rbac_component_min_role_pkey PRIMARY KEY (guild_id, component_key);


--
-- Name: rbac_component_visibility rbac_component_visibility_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rbac_component_visibility
    ADD CONSTRAINT rbac_component_visibility_pkey PRIMARY KEY (guild_id, component_key, role);


--
-- Name: review_queue review_queue_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.review_queue
    ADD CONSTRAINT review_queue_pkey PRIMARY KEY (id);


--
-- Name: role_panel_entries role_panel_entries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.role_panel_entries
    ADD CONSTRAINT role_panel_entries_pkey PRIMARY KEY (id);


--
-- Name: role_panels role_panels_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.role_panels
    ADD CONSTRAINT role_panels_pkey PRIMARY KEY (id);


--
-- Name: rules rules_guild_id_flag_type_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rules
    ADD CONSTRAINT rules_guild_id_flag_type_key UNIQUE (guild_id, flag_type);


--
-- Name: rules rules_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rules
    ADD CONSTRAINT rules_pkey PRIMARY KEY (id);


--
-- Name: sanction_reminders sanction_reminders_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sanction_reminders
    ADD CONSTRAINT sanction_reminders_pkey PRIMARY KEY (id);


--
-- Name: scheduled_announcement_runs scheduled_announcement_runs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.scheduled_announcement_runs
    ADD CONSTRAINT scheduled_announcement_runs_pkey PRIMARY KEY (id);


--
-- Name: scheduled_announcements scheduled_announcements_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.scheduled_announcements
    ADD CONSTRAINT scheduled_announcements_pkey PRIMARY KEY (id);


--
-- Name: security_events security_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.security_events
    ADD CONSTRAINT security_events_pkey PRIMARY KEY (id);


--
-- Name: security_lockdown_active security_lockdown_active_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.security_lockdown_active
    ADD CONSTRAINT security_lockdown_active_pkey PRIMARY KEY (guild_id);


--
-- Name: security_quarantine_pending security_quarantine_pending_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.security_quarantine_pending
    ADD CONSTRAINT security_quarantine_pending_pkey PRIMARY KEY (guild_id, user_id);


--
-- Name: security_slowmode_active security_slowmode_active_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.security_slowmode_active
    ADD CONSTRAINT security_slowmode_active_pkey PRIMARY KEY (guild_id);


--
-- Name: server_events server_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.server_events
    ADD CONSTRAINT server_events_pkey PRIMARY KEY (id);


--
-- Name: sponsorships sponsorships_guild_id_sponsored_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sponsorships
    ADD CONSTRAINT sponsorships_guild_id_sponsored_id_key UNIQUE (guild_id, sponsored_id);


--
-- Name: sponsorships sponsorships_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sponsorships
    ADD CONSTRAINT sponsorships_pkey PRIMARY KEY (id);


--
-- Name: strike_config strike_config_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.strike_config
    ADD CONSTRAINT strike_config_pkey PRIMARY KEY (guild_id);


--
-- Name: successful_logins successful_logins_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.successful_logins
    ADD CONSTRAINT successful_logins_pkey PRIMARY KEY (id);


--
-- Name: temp_roles temp_roles_guild_id_user_id_role_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.temp_roles
    ADD CONSTRAINT temp_roles_guild_id_user_id_role_id_key UNIQUE (guild_id, user_id, role_id);


--
-- Name: temp_roles temp_roles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.temp_roles
    ADD CONSTRAINT temp_roles_pkey PRIMARY KEY (id);


--
-- Name: ticket_assignments ticket_assignments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ticket_assignments
    ADD CONSTRAINT ticket_assignments_pkey PRIMARY KEY (id);


--
-- Name: ticket_messages ticket_messages_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ticket_messages
    ADD CONSTRAINT ticket_messages_pkey PRIMARY KEY (id);


--
-- Name: tickets tickets_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tickets
    ADD CONSTRAINT tickets_pkey PRIMARY KEY (id);


--
-- Name: auto_roles uq_auto_roles_guild_role; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.auto_roles
    ADD CONSTRAINT uq_auto_roles_guild_role UNIQUE (guild_id, role_id);


--
-- Name: daily_activity uq_daily_activity_guild_day; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.daily_activity
    ADD CONSTRAINT uq_daily_activity_guild_day UNIQUE (guild_id, day);


--
-- Name: user_levels_monthly_snapshot uq_levels_monthly_snapshot; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_levels_monthly_snapshot
    ADD CONSTRAINT uq_levels_monthly_snapshot UNIQUE (guild_id, user_id, period_ym);


--
-- Name: user_levels uq_user_levels_guild_user; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_levels
    ADD CONSTRAINT uq_user_levels_guild_user UNIQUE (guild_id, user_id);


--
-- Name: user_stats uq_user_stats_guild_user; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_stats
    ADD CONSTRAINT uq_user_stats_guild_user UNIQUE (guild_id, user_id);


--
-- Name: user_activity_log user_activity_log_pkey1; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log
    ADD CONSTRAINT user_activity_log_pkey1 PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2026_04 user_activity_log_2026_04_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2026_04
    ADD CONSTRAINT user_activity_log_2026_04_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2026_05 user_activity_log_2026_05_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2026_05
    ADD CONSTRAINT user_activity_log_2026_05_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2026_06 user_activity_log_2026_06_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2026_06
    ADD CONSTRAINT user_activity_log_2026_06_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2026_07 user_activity_log_2026_07_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2026_07
    ADD CONSTRAINT user_activity_log_2026_07_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2026_08 user_activity_log_2026_08_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2026_08
    ADD CONSTRAINT user_activity_log_2026_08_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2026_09 user_activity_log_2026_09_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2026_09
    ADD CONSTRAINT user_activity_log_2026_09_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2026_10 user_activity_log_2026_10_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2026_10
    ADD CONSTRAINT user_activity_log_2026_10_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2026_11 user_activity_log_2026_11_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2026_11
    ADD CONSTRAINT user_activity_log_2026_11_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2026_12 user_activity_log_2026_12_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2026_12
    ADD CONSTRAINT user_activity_log_2026_12_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2027_01 user_activity_log_2027_01_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2027_01
    ADD CONSTRAINT user_activity_log_2027_01_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2027_02 user_activity_log_2027_02_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2027_02
    ADD CONSTRAINT user_activity_log_2027_02_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_2027_03 user_activity_log_2027_03_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_2027_03
    ADD CONSTRAINT user_activity_log_2027_03_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_activity_log_default user_activity_log_default_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_activity_log_default
    ADD CONSTRAINT user_activity_log_default_pkey PRIMARY KEY (id, created_at);


--
-- Name: user_cache user_cache_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_cache
    ADD CONSTRAINT user_cache_pkey PRIMARY KEY (guild_id, user_id);


--
-- Name: user_levels user_levels_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_levels
    ADD CONSTRAINT user_levels_pkey PRIMARY KEY (id);


--
-- Name: user_notes user_notes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_notes
    ADD CONSTRAINT user_notes_pkey PRIMARY KEY (id);


--
-- Name: user_stats user_stats_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_stats
    ADD CONSTRAINT user_stats_pkey PRIMARY KEY (id);


--
-- Name: user_strikes user_strikes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_strikes
    ADD CONSTRAINT user_strikes_pkey PRIMARY KEY (id);


--
-- Name: voice_channel_bans voice_channel_bans_owner_user_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_bans
    ADD CONSTRAINT voice_channel_bans_owner_user_key UNIQUE (guild_id, owner_id, user_id);


--
-- Name: voice_channel_bans voice_channel_bans_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_bans
    ADD CONSTRAINT voice_channel_bans_pkey PRIMARY KEY (id);


--
-- Name: voice_channel_co_admins voice_channel_co_admins_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_co_admins
    ADD CONSTRAINT voice_channel_co_admins_pkey PRIMARY KEY (id);


--
-- Name: voice_channel_co_admins voice_channel_co_admins_voice_channel_id_user_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_co_admins
    ADD CONSTRAINT voice_channel_co_admins_voice_channel_id_user_id_key UNIQUE (voice_channel_id, user_id);


--
-- Name: voice_channel_invite_links voice_channel_invite_links_code_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_invite_links
    ADD CONSTRAINT voice_channel_invite_links_code_key UNIQUE (code);


--
-- Name: voice_channel_invite_links voice_channel_invite_links_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_invite_links
    ADD CONSTRAINT voice_channel_invite_links_pkey PRIMARY KEY (id);


--
-- Name: voice_channel_presets voice_channel_presets_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_presets
    ADD CONSTRAINT voice_channel_presets_pkey PRIMARY KEY (guild_id, owner_id);


--
-- Name: voice_channel_themes voice_channel_themes_guild_id_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_themes
    ADD CONSTRAINT voice_channel_themes_guild_id_name_key UNIQUE (guild_id, name);


--
-- Name: voice_channel_themes voice_channel_themes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_themes
    ADD CONSTRAINT voice_channel_themes_pkey PRIMARY KEY (id);


--
-- Name: voice_channel_whitelists voice_channel_whitelists_guild_id_owner_id_target_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_whitelists
    ADD CONSTRAINT voice_channel_whitelists_guild_id_owner_id_target_id_key UNIQUE (guild_id, owner_id, target_id);


--
-- Name: voice_channel_whitelists voice_channel_whitelists_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_whitelists
    ADD CONSTRAINT voice_channel_whitelists_pkey PRIMARY KEY (id);


--
-- Name: voice_channels voice_channels_channel_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channels
    ADD CONSTRAINT voice_channels_channel_id_key UNIQUE (channel_id);


--
-- Name: voice_channels voice_channels_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channels
    ADD CONSTRAINT voice_channels_pkey PRIMARY KEY (id);


--
-- Name: voice_sessions voice_sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_sessions
    ADD CONSTRAINT voice_sessions_pkey PRIMARY KEY (id);


--
-- Name: web_oauth_sessions web_oauth_sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.web_oauth_sessions
    ADD CONSTRAINT web_oauth_sessions_pkey PRIMARY KEY (id);


--
-- Name: welcome_config welcome_config_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.welcome_config
    ADD CONSTRAINT welcome_config_pkey PRIMARY KEY (guild_id);


--
-- Name: idx_audit_logs_actor; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_actor ON ONLY public.audit_logs USING btree (actor_id);


--
-- Name: audit_logs_2026_04_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_04_actor_id_idx ON public.audit_logs_2026_04 USING btree (actor_id);


--
-- Name: idx_audit_logs_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_created_at ON ONLY public.audit_logs USING btree (created_at DESC);


--
-- Name: audit_logs_2026_04_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_04_created_at_idx ON public.audit_logs_2026_04 USING btree (created_at DESC);


--
-- Name: idx_audit_logs_event_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_event_type ON ONLY public.audit_logs USING btree (event_type);


--
-- Name: audit_logs_2026_04_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_04_event_type_idx ON public.audit_logs_2026_04 USING btree (event_type);


--
-- Name: idx_audit_logs_guild_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_guild_created ON ONLY public.audit_logs USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2026_04_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_04_guild_id_created_at_idx ON public.audit_logs_2026_04 USING btree (guild_id, created_at DESC);


--
-- Name: idx_audit_logs_guild_type_date; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_guild_type_date ON ONLY public.audit_logs USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2026_04_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_04_guild_id_event_type_created_at_idx ON public.audit_logs_2026_04 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: idx_audit_logs_target; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_target ON ONLY public.audit_logs USING btree (target_id);


--
-- Name: audit_logs_2026_04_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_04_target_id_idx ON public.audit_logs_2026_04 USING btree (target_id);


--
-- Name: audit_logs_2026_05_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_05_actor_id_idx ON public.audit_logs_2026_05 USING btree (actor_id);


--
-- Name: audit_logs_2026_05_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_05_created_at_idx ON public.audit_logs_2026_05 USING btree (created_at DESC);


--
-- Name: audit_logs_2026_05_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_05_event_type_idx ON public.audit_logs_2026_05 USING btree (event_type);


--
-- Name: audit_logs_2026_05_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_05_guild_id_created_at_idx ON public.audit_logs_2026_05 USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2026_05_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_05_guild_id_event_type_created_at_idx ON public.audit_logs_2026_05 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2026_05_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_05_target_id_idx ON public.audit_logs_2026_05 USING btree (target_id);


--
-- Name: audit_logs_2026_06_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_06_actor_id_idx ON public.audit_logs_2026_06 USING btree (actor_id);


--
-- Name: audit_logs_2026_06_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_06_created_at_idx ON public.audit_logs_2026_06 USING btree (created_at DESC);


--
-- Name: audit_logs_2026_06_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_06_event_type_idx ON public.audit_logs_2026_06 USING btree (event_type);


--
-- Name: audit_logs_2026_06_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_06_guild_id_created_at_idx ON public.audit_logs_2026_06 USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2026_06_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_06_guild_id_event_type_created_at_idx ON public.audit_logs_2026_06 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2026_06_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_06_target_id_idx ON public.audit_logs_2026_06 USING btree (target_id);


--
-- Name: audit_logs_2026_07_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_07_actor_id_idx ON public.audit_logs_2026_07 USING btree (actor_id);


--
-- Name: audit_logs_2026_07_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_07_created_at_idx ON public.audit_logs_2026_07 USING btree (created_at DESC);


--
-- Name: audit_logs_2026_07_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_07_event_type_idx ON public.audit_logs_2026_07 USING btree (event_type);


--
-- Name: audit_logs_2026_07_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_07_guild_id_created_at_idx ON public.audit_logs_2026_07 USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2026_07_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_07_guild_id_event_type_created_at_idx ON public.audit_logs_2026_07 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2026_07_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_07_target_id_idx ON public.audit_logs_2026_07 USING btree (target_id);


--
-- Name: audit_logs_2026_08_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_08_actor_id_idx ON public.audit_logs_2026_08 USING btree (actor_id);


--
-- Name: audit_logs_2026_08_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_08_created_at_idx ON public.audit_logs_2026_08 USING btree (created_at DESC);


--
-- Name: audit_logs_2026_08_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_08_event_type_idx ON public.audit_logs_2026_08 USING btree (event_type);


--
-- Name: audit_logs_2026_08_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_08_guild_id_created_at_idx ON public.audit_logs_2026_08 USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2026_08_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_08_guild_id_event_type_created_at_idx ON public.audit_logs_2026_08 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2026_08_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_08_target_id_idx ON public.audit_logs_2026_08 USING btree (target_id);


--
-- Name: audit_logs_2026_09_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_09_actor_id_idx ON public.audit_logs_2026_09 USING btree (actor_id);


--
-- Name: audit_logs_2026_09_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_09_created_at_idx ON public.audit_logs_2026_09 USING btree (created_at DESC);


--
-- Name: audit_logs_2026_09_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_09_event_type_idx ON public.audit_logs_2026_09 USING btree (event_type);


--
-- Name: audit_logs_2026_09_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_09_guild_id_created_at_idx ON public.audit_logs_2026_09 USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2026_09_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_09_guild_id_event_type_created_at_idx ON public.audit_logs_2026_09 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2026_09_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_09_target_id_idx ON public.audit_logs_2026_09 USING btree (target_id);


--
-- Name: audit_logs_2026_10_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_10_actor_id_idx ON public.audit_logs_2026_10 USING btree (actor_id);


--
-- Name: audit_logs_2026_10_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_10_created_at_idx ON public.audit_logs_2026_10 USING btree (created_at DESC);


--
-- Name: audit_logs_2026_10_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_10_event_type_idx ON public.audit_logs_2026_10 USING btree (event_type);


--
-- Name: audit_logs_2026_10_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_10_guild_id_created_at_idx ON public.audit_logs_2026_10 USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2026_10_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_10_guild_id_event_type_created_at_idx ON public.audit_logs_2026_10 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2026_10_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_10_target_id_idx ON public.audit_logs_2026_10 USING btree (target_id);


--
-- Name: audit_logs_2026_11_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_11_actor_id_idx ON public.audit_logs_2026_11 USING btree (actor_id);


--
-- Name: audit_logs_2026_11_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_11_created_at_idx ON public.audit_logs_2026_11 USING btree (created_at DESC);


--
-- Name: audit_logs_2026_11_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_11_event_type_idx ON public.audit_logs_2026_11 USING btree (event_type);


--
-- Name: audit_logs_2026_11_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_11_guild_id_created_at_idx ON public.audit_logs_2026_11 USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2026_11_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_11_guild_id_event_type_created_at_idx ON public.audit_logs_2026_11 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2026_11_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_11_target_id_idx ON public.audit_logs_2026_11 USING btree (target_id);


--
-- Name: audit_logs_2026_12_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_12_actor_id_idx ON public.audit_logs_2026_12 USING btree (actor_id);


--
-- Name: audit_logs_2026_12_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_12_created_at_idx ON public.audit_logs_2026_12 USING btree (created_at DESC);


--
-- Name: audit_logs_2026_12_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_12_event_type_idx ON public.audit_logs_2026_12 USING btree (event_type);


--
-- Name: audit_logs_2026_12_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_12_guild_id_created_at_idx ON public.audit_logs_2026_12 USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2026_12_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_12_guild_id_event_type_created_at_idx ON public.audit_logs_2026_12 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2026_12_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2026_12_target_id_idx ON public.audit_logs_2026_12 USING btree (target_id);


--
-- Name: audit_logs_2027_01_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_01_actor_id_idx ON public.audit_logs_2027_01 USING btree (actor_id);


--
-- Name: audit_logs_2027_01_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_01_created_at_idx ON public.audit_logs_2027_01 USING btree (created_at DESC);


--
-- Name: audit_logs_2027_01_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_01_event_type_idx ON public.audit_logs_2027_01 USING btree (event_type);


--
-- Name: audit_logs_2027_01_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_01_guild_id_created_at_idx ON public.audit_logs_2027_01 USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2027_01_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_01_guild_id_event_type_created_at_idx ON public.audit_logs_2027_01 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2027_01_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_01_target_id_idx ON public.audit_logs_2027_01 USING btree (target_id);


--
-- Name: audit_logs_2027_02_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_02_actor_id_idx ON public.audit_logs_2027_02 USING btree (actor_id);


--
-- Name: audit_logs_2027_02_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_02_created_at_idx ON public.audit_logs_2027_02 USING btree (created_at DESC);


--
-- Name: audit_logs_2027_02_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_02_event_type_idx ON public.audit_logs_2027_02 USING btree (event_type);


--
-- Name: audit_logs_2027_02_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_02_guild_id_created_at_idx ON public.audit_logs_2027_02 USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2027_02_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_02_guild_id_event_type_created_at_idx ON public.audit_logs_2027_02 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2027_02_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_02_target_id_idx ON public.audit_logs_2027_02 USING btree (target_id);


--
-- Name: audit_logs_2027_03_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_03_actor_id_idx ON public.audit_logs_2027_03 USING btree (actor_id);


--
-- Name: audit_logs_2027_03_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_03_created_at_idx ON public.audit_logs_2027_03 USING btree (created_at DESC);


--
-- Name: audit_logs_2027_03_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_03_event_type_idx ON public.audit_logs_2027_03 USING btree (event_type);


--
-- Name: audit_logs_2027_03_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_03_guild_id_created_at_idx ON public.audit_logs_2027_03 USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_2027_03_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_03_guild_id_event_type_created_at_idx ON public.audit_logs_2027_03 USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_2027_03_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_2027_03_target_id_idx ON public.audit_logs_2027_03 USING btree (target_id);


--
-- Name: audit_logs_default_actor_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_default_actor_id_idx ON public.audit_logs_default USING btree (actor_id);


--
-- Name: audit_logs_default_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_default_created_at_idx ON public.audit_logs_default USING btree (created_at DESC);


--
-- Name: audit_logs_default_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_default_event_type_idx ON public.audit_logs_default USING btree (event_type);


--
-- Name: audit_logs_default_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_default_guild_id_created_at_idx ON public.audit_logs_default USING btree (guild_id, created_at DESC);


--
-- Name: audit_logs_default_guild_id_event_type_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_default_guild_id_event_type_created_at_idx ON public.audit_logs_default USING btree (guild_id, event_type, created_at DESC);


--
-- Name: audit_logs_default_target_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_default_target_id_idx ON public.audit_logs_default USING btree (target_id);


--
-- Name: idx_admin_rotation_history_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_admin_rotation_history_guild ON public.admin_rotation_history USING btree (guild_id, served_at DESC);


--
-- Name: idx_age_bans_guild_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_age_bans_guild_user ON public.age_verification_bans USING btree (guild_id, user_id);


--
-- Name: idx_age_bans_pending_unban; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_age_bans_pending_unban ON public.age_verification_bans USING btree (unban_at) WHERE (status = 'pending'::text);


--
-- Name: idx_ai_dataset_messages_guild_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_ai_dataset_messages_guild_created ON public.ai_dataset_messages USING btree (guild_id, created_at DESC);


--
-- Name: idx_ai_dataset_messages_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_ai_dataset_messages_user ON public.ai_dataset_messages USING btree (guild_id, user_id);


--
-- Name: idx_ai_jobs_guild_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_ai_jobs_guild_created ON public.ai_jobs USING btree (guild_id, created_at DESC);


--
-- Name: idx_ai_jobs_pending; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_ai_jobs_pending ON public.ai_jobs USING btree (created_at) WHERE (status = 'pending'::text);


--
-- Name: idx_ai_jobs_processing; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_ai_jobs_processing ON public.ai_jobs USING btree (started_at) WHERE (status = 'processing'::text);


--
-- Name: idx_analytics_baseline_day; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_analytics_baseline_day ON public.analytics_daily_baseline USING btree (day DESC);


--
-- Name: idx_announcement_runs_announcement; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_announcement_runs_announcement ON public.scheduled_announcement_runs USING btree (announcement_id, ran_at DESC);


--
-- Name: idx_announcement_runs_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_announcement_runs_guild ON public.scheduled_announcement_runs USING btree (guild_id, ran_at DESC);


--
-- Name: idx_announcements_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_announcements_guild ON public.scheduled_announcements USING btree (guild_id, created_at DESC);


--
-- Name: idx_announcements_next_run; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_announcements_next_run ON public.scheduled_announcements USING btree (next_run_at) WHERE (enabled = true);


--
-- Name: idx_api_user_guilds_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_api_user_guilds_guild ON public.api_user_guilds USING btree (guild_id, role);


--
-- Name: idx_auto_roles_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_auto_roles_guild ON public.auto_roles USING btree (guild_id);


--
-- Name: idx_automod_disc_msgs_review; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_automod_disc_msgs_review ON public.automod_discussion_messages USING btree (review_id, sent_at);



--
-- Name: idx_automod_review_votes_review; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_automod_review_votes_review ON public.automod_review_votes USING btree (review_id);


--
-- Name: idx_automod_reviews_active_agg; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_automod_reviews_active_agg ON public.automod_reviews USING btree (guild_id, user_id, last_incident_at DESC) WHERE (status = 'voting'::text);


--
-- Name: idx_automod_reviews_flags_gin; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_automod_reviews_flags_gin ON public.automod_reviews USING gin (flags);


--
-- Name: idx_automod_reviews_guild_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_automod_reviews_guild_status ON public.automod_reviews USING btree (guild_id, status, created_at DESC);


--
-- Name: idx_automod_reviews_guild_user_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_automod_reviews_guild_user_status ON public.automod_reviews USING btree (guild_id, user_id, status, created_at DESC);


--
-- Name: idx_automod_reviews_open_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_automod_reviews_open_user ON public.automod_reviews USING btree (guild_id, user_id) WHERE (status = 'voting'::text);


--
-- Name: idx_automod_reviews_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_automod_reviews_user ON public.automod_reviews USING btree (user_id);


--
-- Name: idx_automod_reviews_voting_deadline; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_automod_reviews_voting_deadline ON public.automod_reviews USING btree (voting_deadline) WHERE (status = 'voting'::text);


--
-- Name: idx_bot_definitions_config_schema_gin; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_bot_definitions_config_schema_gin ON public.bot_definitions USING gin (config_schema);


--
-- Name: idx_bot_guild_config_bot; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_bot_guild_config_bot ON public.bot_guild_config USING btree (guild_id, bot_name);


--
-- Name: idx_bot_guild_config_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_bot_guild_config_guild ON public.bot_guild_config USING btree (guild_id);


--
-- Name: idx_bump_events_user_week; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_bump_events_user_week ON public.bump_events USING btree (guild_id, user_id, bumped_at DESC);


--
-- Name: idx_button_interactions_announcement; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_button_interactions_announcement ON public.announcement_button_interactions USING btree (announcement_id, clicked_at DESC);


--
-- Name: idx_button_interactions_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_button_interactions_user ON public.announcement_button_interactions USING btree (user_id, clicked_at DESC);



--
-- Name: idx_confession_replies_message; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_confession_replies_message ON public.confession_replies USING btree (message_id) WHERE (message_id IS NOT NULL);


--
-- Name: idx_confession_reports_guild_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_confession_reports_guild_status ON public.confession_reports USING btree (guild_id, status, created_at DESC);


--
-- Name: idx_confessions_author; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_confessions_author ON public.confessions USING btree (guild_id, author_user_id, created_at DESC);



--
-- Name: idx_confessions_message; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_confessions_message ON public.confessions USING btree (message_id) WHERE (message_id IS NOT NULL);



--
-- Name: idx_dam_action; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_dam_action ON public.discord_action_messages USING btree (action_id);


--
-- Name: idx_dam_kind_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_dam_kind_guild ON public.discord_action_messages USING btree (kind, guild_id);


--
-- Name: idx_discord_roles_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_discord_roles_guild ON public.discord_roles USING btree (guild_id, "position" DESC);


--
-- Name: idx_evidence_action; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_evidence_action ON public.moderation_evidence USING btree (action_id);


--
-- Name: idx_evidence_uploaded_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_evidence_uploaded_at ON public.moderation_evidence USING btree (uploaded_at DESC);


--
-- Name: idx_export_jobs_guild_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_export_jobs_guild_created ON public.export_jobs USING btree (guild_id, created_at DESC);


--
-- Name: idx_export_jobs_pending; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_export_jobs_pending ON public.export_jobs USING btree (created_at) WHERE (status = 'pending'::text);


--
-- Name: idx_export_jobs_processing; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_export_jobs_processing ON public.export_jobs USING btree (started_at) WHERE (status = 'processing'::text);


--
-- Name: idx_guild_members_active; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_guild_members_active ON public.guild_members USING btree (guild_id, user_id) WHERE (left_at IS NULL);


--
-- Name: idx_guild_members_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_guild_members_guild ON public.guild_members USING btree (guild_id);


--
-- Name: idx_guild_members_username; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_guild_members_username ON public.guild_members USING btree (guild_id, username);


--
-- Name: idx_guild_snapshots_guild_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_guild_snapshots_guild_created ON public.guild_snapshots USING btree (guild_id, created_at DESC);


--
-- Name: idx_hourly_activity_day; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_hourly_activity_day ON public.hourly_activity USING btree (day DESC);


--
-- Name: idx_hourly_activity_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_hourly_activity_guild ON public.hourly_activity USING btree (guild_id);


--
-- Name: idx_infractions_flags_gin; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_infractions_flags_gin ON ONLY public.infractions USING gin (flags);


--
-- Name: idx_infractions_guild_action; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_infractions_guild_action ON ONLY public.infractions USING btree (guild_id, action);


--
-- Name: idx_infractions_guild_action_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_infractions_guild_action_created ON ONLY public.infractions USING btree (guild_id, action, created_at DESC);


--
-- Name: idx_infractions_guild_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_infractions_guild_created ON ONLY public.infractions USING btree (guild_id, created_at DESC);


--
-- Name: idx_infractions_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_infractions_user ON ONLY public.infractions USING btree (guild_id, user_id);


--
-- Name: idx_invitation_codes_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_invitation_codes_guild ON public.invitation_codes USING btree (guild_id);


--
-- Name: idx_invitation_codes_unused; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_invitation_codes_unused ON public.invitation_codes USING btree (used_at) WHERE (used_at IS NULL);


--
-- Name: idx_invite_links_channel; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_invite_links_channel ON public.voice_channel_invite_links USING btree (channel_id);


--
-- Name: idx_invite_links_expires; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_invite_links_expires ON public.voice_channel_invite_links USING btree (expires_at);


--
-- Name: idx_invite_links_voice_channel; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_invite_links_voice_channel ON public.voice_channel_invite_links USING btree (voice_channel_id);


--
-- Name: idx_levels_monthly_snapshot_period; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_levels_monthly_snapshot_period ON public.user_levels_monthly_snapshot USING btree (guild_id, period_ym);


--
-- Name: idx_logs_bot; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_logs_bot ON ONLY public.logs USING btree (bot);


--
-- Name: idx_logs_level; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_logs_level ON ONLY public.logs USING btree (level);


--
-- Name: idx_logs_timestamp; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_logs_timestamp ON ONLY public.logs USING btree ("timestamp" DESC);


--
-- Name: idx_manual_ip_bans_active; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_manual_ip_bans_active ON public.manual_ip_bans USING btree (banned_at DESC) WHERE (unbanned_at IS NULL);


--
-- Name: idx_manual_watched_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_manual_watched_guild ON public.manual_watched_users USING btree (guild_id);


--
-- Name: idx_mod_actions_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_mod_actions_created ON public.moderation_actions USING btree (created_at DESC);


--
-- Name: idx_mod_actions_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_mod_actions_guild ON public.moderation_actions USING btree (guild_id);


--
-- Name: idx_mod_actions_guild_type_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_mod_actions_guild_type_created ON public.moderation_actions USING btree (guild_id, action_type, created_at DESC);


--
-- Name: idx_mod_actions_target; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_mod_actions_target ON public.moderation_actions USING btree (guild_id, target_id);


--
-- Name: idx_mod_actions_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_mod_actions_type ON public.moderation_actions USING btree (action_type);


--
-- Name: idx_moderation_sursis_due; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_moderation_sursis_due ON public.moderation_sursis USING btree (expires_at) WHERE (status = 'en_sursis'::text);


--
-- Name: idx_mv_level_leaderboard_rank; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_mv_level_leaderboard_rank ON public.mv_level_leaderboard USING btree (guild_id, rank);


--
-- Name: idx_notes_guild_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_notes_guild_user ON public.user_notes USING btree (guild_id, user_id);


--
-- Name: idx_pending_mod_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_pending_mod_guild ON public.pending_mod_actions USING btree (guild_id);


--
-- Name: idx_pending_mod_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_pending_mod_status ON public.pending_mod_actions USING btree (guild_id, status);


--
-- Name: idx_rbac_component_min_role_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_rbac_component_min_role_guild ON public.rbac_component_min_role USING btree (guild_id);


--
-- Name: idx_rbac_visibility_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_rbac_visibility_guild ON public.rbac_component_visibility USING btree (guild_id);


--
-- Name: idx_reminders_action; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reminders_action ON public.sanction_reminders USING btree (action_id);


--
-- Name: idx_reminders_pending; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reminders_pending ON public.sanction_reminders USING btree (remind_at) WHERE (status = 'pending'::text);


--
-- Name: idx_reminders_unban_pending; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reminders_unban_pending ON public.sanction_reminders USING btree (expires_at) WHERE (unban_status = 'pending'::text);


--
-- Name: idx_review_queue_action; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_review_queue_action ON public.review_queue USING btree (action_id);


--
-- Name: idx_review_queue_pending; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_review_queue_pending ON public.review_queue USING btree (guild_id, added_at DESC) WHERE (status = 'pending'::text);


--
-- Name: idx_role_panel_entries_panel; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_role_panel_entries_panel ON public.role_panel_entries USING btree (panel_id);


--
-- Name: idx_role_panels_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_role_panels_guild ON public.role_panels USING btree (guild_id);


--
-- Name: idx_role_panels_message; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_role_panels_message ON public.role_panels USING btree (message_id);


--
-- Name: idx_rules_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_rules_guild ON public.rules USING btree (guild_id);


--
-- Name: idx_security_events_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_security_events_created ON public.security_events USING btree (created_at DESC);


--
-- Name: idx_security_events_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_security_events_guild ON public.security_events USING btree (guild_id);


--
-- Name: idx_security_events_severity; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_security_events_severity ON public.security_events USING btree (severity);


--
-- Name: idx_security_events_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_security_events_type ON public.security_events USING btree (event_type);


--
-- Name: idx_security_events_user_ids_gin; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_security_events_user_ids_gin ON public.security_events USING gin (user_ids);


--
-- Name: idx_security_lockdown_expires; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_security_lockdown_expires ON public.security_lockdown_active USING btree (expires_at);


--
-- Name: idx_security_quarantine_expires; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_security_quarantine_expires ON public.security_quarantine_pending USING btree (expires_at);


--
-- Name: idx_security_slowmode_expires; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_security_slowmode_expires ON public.security_slowmode_active USING btree (expires_at);


--
-- Name: idx_server_events_action; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_server_events_action ON public.server_events USING btree (action);


--
-- Name: idx_server_events_actor; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_server_events_actor ON public.server_events USING btree (actor);


--
-- Name: idx_server_events_severity; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_server_events_severity ON public.server_events USING btree (severity);


--
-- Name: idx_server_events_ts; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_server_events_ts ON public.server_events USING btree ("timestamp" DESC);


--
-- Name: idx_sponsorships_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sponsorships_guild ON public.sponsorships USING btree (guild_id);


--
-- Name: idx_sponsorships_sponsor; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sponsorships_sponsor ON public.sponsorships USING btree (guild_id, sponsor_id);


--
-- Name: idx_strikes_expires; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_strikes_expires ON public.user_strikes USING btree (expires_at) WHERE (expires_at IS NOT NULL);


--
-- Name: idx_strikes_guild_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_strikes_guild_user ON public.user_strikes USING btree (guild_id, user_id);


--
-- Name: idx_successful_logins_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_successful_logins_at ON public.successful_logins USING btree (logged_at DESC);


--
-- Name: idx_successful_logins_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_successful_logins_user ON public.successful_logins USING btree (discord_user_id);


--
-- Name: idx_temp_roles_expires; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_temp_roles_expires ON public.temp_roles USING btree (expires_at);


--
-- Name: idx_temp_roles_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_temp_roles_guild ON public.temp_roles USING btree (guild_id);


--
-- Name: idx_ticket_assignments_ticket; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_ticket_assignments_ticket ON public.ticket_assignments USING btree (ticket_id);


--
-- Name: idx_ticket_messages_ticket; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_ticket_messages_ticket ON public.ticket_messages USING btree (ticket_id);


--
-- Name: idx_tickets_appeal_sla_pending; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tickets_appeal_sla_pending ON public.tickets USING btree (created_at, server) WHERE ((category = 'appel_sanction'::text) AND (status = ANY (ARRAY['open'::text, 'assigned'::text])) AND (escalated_at IS NULL) AND (first_response_at IS NULL));


--
-- Name: idx_tickets_assigned; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tickets_assigned ON public.tickets USING btree (assigned_to) WHERE (assigned_to IS NOT NULL);


--
-- Name: idx_tickets_author; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tickets_author ON public.tickets USING btree (author_id);


--
-- Name: idx_tickets_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tickets_created ON public.tickets USING btree (created_at DESC);


--
-- Name: idx_tickets_guild_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tickets_guild_id ON public.tickets USING btree (guild_id);


--
-- Name: idx_tickets_open; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tickets_open ON public.tickets USING btree (server, created_at DESC) WHERE (status = ANY (ARRAY['open'::text, 'assigned'::text]));


--
-- Name: idx_tickets_server; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tickets_server ON public.tickets USING btree (server);


--
-- Name: idx_tickets_sla_warning; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_tickets_sla_warning ON public.tickets USING btree (server, created_at) WHERE ((first_response_at IS NULL) AND (sla_warned_at IS NULL) AND (escalated_at IS NULL) AND (status = ANY (ARRAY['open'::text, 'assigned'::text])) AND (category <> 'appel_sanction'::text));


--
-- Name: idx_user_activity_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_activity_created ON ONLY public.user_activity_log USING btree (created_at);


--
-- Name: idx_user_activity_guild_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_activity_guild_user ON ONLY public.user_activity_log USING btree (guild_id, user_id);


--
-- Name: idx_user_activity_guild_user_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_activity_guild_user_type ON ONLY public.user_activity_log USING btree (guild_id, user_id, event_type);


--
-- Name: idx_user_activity_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_activity_type ON ONLY public.user_activity_log USING btree (event_type);


--
-- Name: idx_user_cache_updated; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_cache_updated ON public.user_cache USING btree (updated_at DESC);


--
-- Name: idx_user_levels_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_levels_guild ON public.user_levels USING btree (guild_id);


--
-- Name: idx_user_levels_guild_level; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_levels_guild_level ON public.user_levels USING btree (guild_id, level DESC);


--
-- Name: idx_user_levels_guild_xp; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_levels_guild_xp ON public.user_levels USING btree (guild_id, xp DESC);


--
-- Name: idx_user_levels_xp_text; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_levels_xp_text ON public.user_levels USING btree (guild_id, xp_text DESC);


--
-- Name: idx_user_levels_xp_voice; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_levels_xp_voice ON public.user_levels USING btree (guild_id, xp_voice DESC);


--
-- Name: idx_user_stats_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_stats_guild ON public.user_stats USING btree (guild_id);



--
-- Name: idx_voice_bans_channel; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_bans_channel ON public.voice_channel_bans USING btree (voice_channel_id);


--
-- Name: idx_voice_bans_expires; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_bans_expires ON public.voice_channel_bans USING btree (expires_at) WHERE (expires_at IS NOT NULL);


--
-- Name: idx_voice_bans_owner; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_bans_owner ON public.voice_channel_bans USING btree (guild_id, owner_id);


--
-- Name: idx_voice_channels_active; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_channels_active ON public.voice_channels USING btree (guild_id, owner_id) WHERE ((channel_status)::text = 'open'::text);


--
-- Name: idx_voice_channels_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_channels_guild ON public.voice_channels USING btree (guild_id);


--
-- Name: idx_voice_channels_owner; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_channels_owner ON public.voice_channels USING btree (owner_id);


--
-- Name: idx_voice_co_admins_channel; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_co_admins_channel ON public.voice_channel_co_admins USING btree (voice_channel_id);


--
-- Name: idx_voice_sessions_channel; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_sessions_channel ON public.voice_sessions USING btree (channel_id);


--
-- Name: idx_voice_sessions_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_sessions_guild ON public.voice_sessions USING btree (guild_id);


--
-- Name: idx_voice_sessions_started; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_sessions_started ON public.voice_sessions USING btree (started_at);


--
-- Name: idx_voice_sessions_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_sessions_user ON public.voice_sessions USING btree (guild_id, user_id);


--
-- Name: idx_voice_themes_guild; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_themes_guild ON public.voice_channel_themes USING btree (guild_id);


--
-- Name: idx_voice_whitelists_owner; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_voice_whitelists_owner ON public.voice_channel_whitelists USING btree (guild_id, owner_id);


--
-- Name: idx_web_oauth_sessions_last_used; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_web_oauth_sessions_last_used ON public.web_oauth_sessions USING btree (last_used_at);


--
-- Name: idx_web_oauth_sessions_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_web_oauth_sessions_user ON public.web_oauth_sessions USING btree (discord_user_id);


--
-- Name: infractions_2026_04_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_04_flags_idx ON public.infractions_2026_04 USING gin (flags);


--
-- Name: infractions_2026_04_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_04_guild_id_action_created_at_idx ON public.infractions_2026_04 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2026_04_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_04_guild_id_action_idx ON public.infractions_2026_04 USING btree (guild_id, action);


--
-- Name: infractions_2026_04_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_04_guild_id_created_at_idx ON public.infractions_2026_04 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2026_04_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_04_guild_id_user_id_idx ON public.infractions_2026_04 USING btree (guild_id, user_id);


--
-- Name: infractions_2026_05_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_05_flags_idx ON public.infractions_2026_05 USING gin (flags);


--
-- Name: infractions_2026_05_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_05_guild_id_action_created_at_idx ON public.infractions_2026_05 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2026_05_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_05_guild_id_action_idx ON public.infractions_2026_05 USING btree (guild_id, action);


--
-- Name: infractions_2026_05_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_05_guild_id_created_at_idx ON public.infractions_2026_05 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2026_05_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_05_guild_id_user_id_idx ON public.infractions_2026_05 USING btree (guild_id, user_id);


--
-- Name: infractions_2026_06_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_06_flags_idx ON public.infractions_2026_06 USING gin (flags);


--
-- Name: infractions_2026_06_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_06_guild_id_action_created_at_idx ON public.infractions_2026_06 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2026_06_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_06_guild_id_action_idx ON public.infractions_2026_06 USING btree (guild_id, action);


--
-- Name: infractions_2026_06_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_06_guild_id_created_at_idx ON public.infractions_2026_06 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2026_06_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_06_guild_id_user_id_idx ON public.infractions_2026_06 USING btree (guild_id, user_id);


--
-- Name: infractions_2026_07_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_07_flags_idx ON public.infractions_2026_07 USING gin (flags);


--
-- Name: infractions_2026_07_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_07_guild_id_action_created_at_idx ON public.infractions_2026_07 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2026_07_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_07_guild_id_action_idx ON public.infractions_2026_07 USING btree (guild_id, action);


--
-- Name: infractions_2026_07_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_07_guild_id_created_at_idx ON public.infractions_2026_07 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2026_07_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_07_guild_id_user_id_idx ON public.infractions_2026_07 USING btree (guild_id, user_id);


--
-- Name: infractions_2026_08_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_08_flags_idx ON public.infractions_2026_08 USING gin (flags);


--
-- Name: infractions_2026_08_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_08_guild_id_action_created_at_idx ON public.infractions_2026_08 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2026_08_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_08_guild_id_action_idx ON public.infractions_2026_08 USING btree (guild_id, action);


--
-- Name: infractions_2026_08_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_08_guild_id_created_at_idx ON public.infractions_2026_08 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2026_08_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_08_guild_id_user_id_idx ON public.infractions_2026_08 USING btree (guild_id, user_id);


--
-- Name: infractions_2026_09_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_09_flags_idx ON public.infractions_2026_09 USING gin (flags);


--
-- Name: infractions_2026_09_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_09_guild_id_action_created_at_idx ON public.infractions_2026_09 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2026_09_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_09_guild_id_action_idx ON public.infractions_2026_09 USING btree (guild_id, action);


--
-- Name: infractions_2026_09_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_09_guild_id_created_at_idx ON public.infractions_2026_09 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2026_09_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_09_guild_id_user_id_idx ON public.infractions_2026_09 USING btree (guild_id, user_id);


--
-- Name: infractions_2026_10_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_10_flags_idx ON public.infractions_2026_10 USING gin (flags);


--
-- Name: infractions_2026_10_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_10_guild_id_action_created_at_idx ON public.infractions_2026_10 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2026_10_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_10_guild_id_action_idx ON public.infractions_2026_10 USING btree (guild_id, action);


--
-- Name: infractions_2026_10_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_10_guild_id_created_at_idx ON public.infractions_2026_10 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2026_10_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_10_guild_id_user_id_idx ON public.infractions_2026_10 USING btree (guild_id, user_id);


--
-- Name: infractions_2026_11_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_11_flags_idx ON public.infractions_2026_11 USING gin (flags);


--
-- Name: infractions_2026_11_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_11_guild_id_action_created_at_idx ON public.infractions_2026_11 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2026_11_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_11_guild_id_action_idx ON public.infractions_2026_11 USING btree (guild_id, action);


--
-- Name: infractions_2026_11_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_11_guild_id_created_at_idx ON public.infractions_2026_11 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2026_11_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_11_guild_id_user_id_idx ON public.infractions_2026_11 USING btree (guild_id, user_id);


--
-- Name: infractions_2026_12_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_12_flags_idx ON public.infractions_2026_12 USING gin (flags);


--
-- Name: infractions_2026_12_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_12_guild_id_action_created_at_idx ON public.infractions_2026_12 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2026_12_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_12_guild_id_action_idx ON public.infractions_2026_12 USING btree (guild_id, action);


--
-- Name: infractions_2026_12_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_12_guild_id_created_at_idx ON public.infractions_2026_12 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2026_12_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2026_12_guild_id_user_id_idx ON public.infractions_2026_12 USING btree (guild_id, user_id);


--
-- Name: infractions_2027_01_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_01_flags_idx ON public.infractions_2027_01 USING gin (flags);


--
-- Name: infractions_2027_01_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_01_guild_id_action_created_at_idx ON public.infractions_2027_01 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2027_01_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_01_guild_id_action_idx ON public.infractions_2027_01 USING btree (guild_id, action);


--
-- Name: infractions_2027_01_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_01_guild_id_created_at_idx ON public.infractions_2027_01 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2027_01_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_01_guild_id_user_id_idx ON public.infractions_2027_01 USING btree (guild_id, user_id);


--
-- Name: infractions_2027_02_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_02_flags_idx ON public.infractions_2027_02 USING gin (flags);


--
-- Name: infractions_2027_02_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_02_guild_id_action_created_at_idx ON public.infractions_2027_02 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2027_02_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_02_guild_id_action_idx ON public.infractions_2027_02 USING btree (guild_id, action);


--
-- Name: infractions_2027_02_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_02_guild_id_created_at_idx ON public.infractions_2027_02 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2027_02_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_02_guild_id_user_id_idx ON public.infractions_2027_02 USING btree (guild_id, user_id);


--
-- Name: infractions_2027_03_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_03_flags_idx ON public.infractions_2027_03 USING gin (flags);


--
-- Name: infractions_2027_03_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_03_guild_id_action_created_at_idx ON public.infractions_2027_03 USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_2027_03_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_03_guild_id_action_idx ON public.infractions_2027_03 USING btree (guild_id, action);


--
-- Name: infractions_2027_03_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_03_guild_id_created_at_idx ON public.infractions_2027_03 USING btree (guild_id, created_at DESC);


--
-- Name: infractions_2027_03_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_2027_03_guild_id_user_id_idx ON public.infractions_2027_03 USING btree (guild_id, user_id);


--
-- Name: infractions_default_flags_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_default_flags_idx ON public.infractions_default USING gin (flags);


--
-- Name: infractions_default_guild_id_action_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_default_guild_id_action_created_at_idx ON public.infractions_default USING btree (guild_id, action, created_at DESC);


--
-- Name: infractions_default_guild_id_action_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_default_guild_id_action_idx ON public.infractions_default USING btree (guild_id, action);


--
-- Name: infractions_default_guild_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_default_guild_id_created_at_idx ON public.infractions_default USING btree (guild_id, created_at DESC);


--
-- Name: infractions_default_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX infractions_default_guild_id_user_id_idx ON public.infractions_default USING btree (guild_id, user_id);


--
-- Name: logs_2026_04_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_04_bot_idx ON public.logs_2026_04 USING btree (bot);


--
-- Name: logs_2026_04_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_04_level_idx ON public.logs_2026_04 USING btree (level);


--
-- Name: logs_2026_04_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_04_timestamp_idx ON public.logs_2026_04 USING btree ("timestamp" DESC);


--
-- Name: logs_2026_05_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_05_bot_idx ON public.logs_2026_05 USING btree (bot);


--
-- Name: logs_2026_05_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_05_level_idx ON public.logs_2026_05 USING btree (level);


--
-- Name: logs_2026_05_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_05_timestamp_idx ON public.logs_2026_05 USING btree ("timestamp" DESC);


--
-- Name: logs_2026_06_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_06_bot_idx ON public.logs_2026_06 USING btree (bot);


--
-- Name: logs_2026_06_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_06_level_idx ON public.logs_2026_06 USING btree (level);


--
-- Name: logs_2026_06_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_06_timestamp_idx ON public.logs_2026_06 USING btree ("timestamp" DESC);


--
-- Name: logs_2026_07_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_07_bot_idx ON public.logs_2026_07 USING btree (bot);


--
-- Name: logs_2026_07_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_07_level_idx ON public.logs_2026_07 USING btree (level);


--
-- Name: logs_2026_07_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_07_timestamp_idx ON public.logs_2026_07 USING btree ("timestamp" DESC);


--
-- Name: logs_2026_08_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_08_bot_idx ON public.logs_2026_08 USING btree (bot);


--
-- Name: logs_2026_08_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_08_level_idx ON public.logs_2026_08 USING btree (level);


--
-- Name: logs_2026_08_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_08_timestamp_idx ON public.logs_2026_08 USING btree ("timestamp" DESC);


--
-- Name: logs_2026_09_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_09_bot_idx ON public.logs_2026_09 USING btree (bot);


--
-- Name: logs_2026_09_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_09_level_idx ON public.logs_2026_09 USING btree (level);


--
-- Name: logs_2026_09_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_09_timestamp_idx ON public.logs_2026_09 USING btree ("timestamp" DESC);


--
-- Name: logs_2026_10_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_10_bot_idx ON public.logs_2026_10 USING btree (bot);


--
-- Name: logs_2026_10_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_10_level_idx ON public.logs_2026_10 USING btree (level);


--
-- Name: logs_2026_10_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_10_timestamp_idx ON public.logs_2026_10 USING btree ("timestamp" DESC);


--
-- Name: logs_2026_11_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_11_bot_idx ON public.logs_2026_11 USING btree (bot);


--
-- Name: logs_2026_11_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_11_level_idx ON public.logs_2026_11 USING btree (level);


--
-- Name: logs_2026_11_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_11_timestamp_idx ON public.logs_2026_11 USING btree ("timestamp" DESC);


--
-- Name: logs_2026_12_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_12_bot_idx ON public.logs_2026_12 USING btree (bot);


--
-- Name: logs_2026_12_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_12_level_idx ON public.logs_2026_12 USING btree (level);


--
-- Name: logs_2026_12_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2026_12_timestamp_idx ON public.logs_2026_12 USING btree ("timestamp" DESC);


--
-- Name: logs_2027_01_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2027_01_bot_idx ON public.logs_2027_01 USING btree (bot);


--
-- Name: logs_2027_01_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2027_01_level_idx ON public.logs_2027_01 USING btree (level);


--
-- Name: logs_2027_01_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2027_01_timestamp_idx ON public.logs_2027_01 USING btree ("timestamp" DESC);


--
-- Name: logs_2027_02_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2027_02_bot_idx ON public.logs_2027_02 USING btree (bot);


--
-- Name: logs_2027_02_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2027_02_level_idx ON public.logs_2027_02 USING btree (level);


--
-- Name: logs_2027_02_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2027_02_timestamp_idx ON public.logs_2027_02 USING btree ("timestamp" DESC);


--
-- Name: logs_2027_03_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2027_03_bot_idx ON public.logs_2027_03 USING btree (bot);


--
-- Name: logs_2027_03_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2027_03_level_idx ON public.logs_2027_03 USING btree (level);


--
-- Name: logs_2027_03_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_2027_03_timestamp_idx ON public.logs_2027_03 USING btree ("timestamp" DESC);


--
-- Name: logs_default_bot_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_default_bot_idx ON public.logs_default USING btree (bot);


--
-- Name: logs_default_level_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_default_level_idx ON public.logs_default USING btree (level);


--
-- Name: logs_default_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX logs_default_timestamp_idx ON public.logs_default USING btree ("timestamp" DESC);


--
-- Name: uq_moderation_sursis_active; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX uq_moderation_sursis_active ON public.moderation_sursis USING btree (guild_id, user_id) WHERE (status = 'en_sursis'::text);


--
-- Name: uq_mv_level_leaderboard; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX uq_mv_level_leaderboard ON public.mv_level_leaderboard USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2026_04_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_04_created_at_idx ON public.user_activity_log_2026_04 USING btree (created_at);


--
-- Name: user_activity_log_2026_04_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_04_event_type_idx ON public.user_activity_log_2026_04 USING btree (event_type);


--
-- Name: user_activity_log_2026_04_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_04_guild_id_user_id_event_type_idx ON public.user_activity_log_2026_04 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2026_04_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_04_guild_id_user_id_idx ON public.user_activity_log_2026_04 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2026_05_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_05_created_at_idx ON public.user_activity_log_2026_05 USING btree (created_at);


--
-- Name: user_activity_log_2026_05_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_05_event_type_idx ON public.user_activity_log_2026_05 USING btree (event_type);


--
-- Name: user_activity_log_2026_05_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_05_guild_id_user_id_event_type_idx ON public.user_activity_log_2026_05 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2026_05_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_05_guild_id_user_id_idx ON public.user_activity_log_2026_05 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2026_06_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_06_created_at_idx ON public.user_activity_log_2026_06 USING btree (created_at);


--
-- Name: user_activity_log_2026_06_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_06_event_type_idx ON public.user_activity_log_2026_06 USING btree (event_type);


--
-- Name: user_activity_log_2026_06_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_06_guild_id_user_id_event_type_idx ON public.user_activity_log_2026_06 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2026_06_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_06_guild_id_user_id_idx ON public.user_activity_log_2026_06 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2026_07_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_07_created_at_idx ON public.user_activity_log_2026_07 USING btree (created_at);


--
-- Name: user_activity_log_2026_07_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_07_event_type_idx ON public.user_activity_log_2026_07 USING btree (event_type);


--
-- Name: user_activity_log_2026_07_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_07_guild_id_user_id_event_type_idx ON public.user_activity_log_2026_07 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2026_07_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_07_guild_id_user_id_idx ON public.user_activity_log_2026_07 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2026_08_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_08_created_at_idx ON public.user_activity_log_2026_08 USING btree (created_at);


--
-- Name: user_activity_log_2026_08_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_08_event_type_idx ON public.user_activity_log_2026_08 USING btree (event_type);


--
-- Name: user_activity_log_2026_08_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_08_guild_id_user_id_event_type_idx ON public.user_activity_log_2026_08 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2026_08_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_08_guild_id_user_id_idx ON public.user_activity_log_2026_08 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2026_09_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_09_created_at_idx ON public.user_activity_log_2026_09 USING btree (created_at);


--
-- Name: user_activity_log_2026_09_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_09_event_type_idx ON public.user_activity_log_2026_09 USING btree (event_type);


--
-- Name: user_activity_log_2026_09_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_09_guild_id_user_id_event_type_idx ON public.user_activity_log_2026_09 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2026_09_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_09_guild_id_user_id_idx ON public.user_activity_log_2026_09 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2026_10_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_10_created_at_idx ON public.user_activity_log_2026_10 USING btree (created_at);


--
-- Name: user_activity_log_2026_10_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_10_event_type_idx ON public.user_activity_log_2026_10 USING btree (event_type);


--
-- Name: user_activity_log_2026_10_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_10_guild_id_user_id_event_type_idx ON public.user_activity_log_2026_10 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2026_10_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_10_guild_id_user_id_idx ON public.user_activity_log_2026_10 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2026_11_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_11_created_at_idx ON public.user_activity_log_2026_11 USING btree (created_at);


--
-- Name: user_activity_log_2026_11_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_11_event_type_idx ON public.user_activity_log_2026_11 USING btree (event_type);


--
-- Name: user_activity_log_2026_11_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_11_guild_id_user_id_event_type_idx ON public.user_activity_log_2026_11 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2026_11_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_11_guild_id_user_id_idx ON public.user_activity_log_2026_11 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2026_12_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_12_created_at_idx ON public.user_activity_log_2026_12 USING btree (created_at);


--
-- Name: user_activity_log_2026_12_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_12_event_type_idx ON public.user_activity_log_2026_12 USING btree (event_type);


--
-- Name: user_activity_log_2026_12_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_12_guild_id_user_id_event_type_idx ON public.user_activity_log_2026_12 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2026_12_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2026_12_guild_id_user_id_idx ON public.user_activity_log_2026_12 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2027_01_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_01_created_at_idx ON public.user_activity_log_2027_01 USING btree (created_at);


--
-- Name: user_activity_log_2027_01_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_01_event_type_idx ON public.user_activity_log_2027_01 USING btree (event_type);


--
-- Name: user_activity_log_2027_01_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_01_guild_id_user_id_event_type_idx ON public.user_activity_log_2027_01 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2027_01_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_01_guild_id_user_id_idx ON public.user_activity_log_2027_01 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2027_02_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_02_created_at_idx ON public.user_activity_log_2027_02 USING btree (created_at);


--
-- Name: user_activity_log_2027_02_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_02_event_type_idx ON public.user_activity_log_2027_02 USING btree (event_type);


--
-- Name: user_activity_log_2027_02_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_02_guild_id_user_id_event_type_idx ON public.user_activity_log_2027_02 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2027_02_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_02_guild_id_user_id_idx ON public.user_activity_log_2027_02 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_2027_03_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_03_created_at_idx ON public.user_activity_log_2027_03 USING btree (created_at);


--
-- Name: user_activity_log_2027_03_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_03_event_type_idx ON public.user_activity_log_2027_03 USING btree (event_type);


--
-- Name: user_activity_log_2027_03_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_03_guild_id_user_id_event_type_idx ON public.user_activity_log_2027_03 USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_2027_03_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_2027_03_guild_id_user_id_idx ON public.user_activity_log_2027_03 USING btree (guild_id, user_id);


--
-- Name: user_activity_log_default_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_default_created_at_idx ON public.user_activity_log_default USING btree (created_at);


--
-- Name: user_activity_log_default_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_default_event_type_idx ON public.user_activity_log_default USING btree (event_type);


--
-- Name: user_activity_log_default_guild_id_user_id_event_type_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_default_guild_id_user_id_event_type_idx ON public.user_activity_log_default USING btree (guild_id, user_id, event_type);


--
-- Name: user_activity_log_default_guild_id_user_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_activity_log_default_guild_id_user_id_idx ON public.user_activity_log_default USING btree (guild_id, user_id);


--
-- Name: ux_user_strikes_infraction; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX ux_user_strikes_infraction ON public.user_strikes USING btree (infraction_id) WHERE (infraction_id IS NOT NULL);


--
-- Name: audit_logs_2026_04_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2026_04_actor_id_idx;


--
-- Name: audit_logs_2026_04_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2026_04_created_at_idx;


--
-- Name: audit_logs_2026_04_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2026_04_event_type_idx;


--
-- Name: audit_logs_2026_04_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2026_04_guild_id_created_at_idx;


--
-- Name: audit_logs_2026_04_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2026_04_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2026_04_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2026_04_pkey;


--
-- Name: audit_logs_2026_04_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2026_04_target_id_idx;


--
-- Name: audit_logs_2026_05_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2026_05_actor_id_idx;


--
-- Name: audit_logs_2026_05_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2026_05_created_at_idx;


--
-- Name: audit_logs_2026_05_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2026_05_event_type_idx;


--
-- Name: audit_logs_2026_05_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2026_05_guild_id_created_at_idx;


--
-- Name: audit_logs_2026_05_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2026_05_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2026_05_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2026_05_pkey;


--
-- Name: audit_logs_2026_05_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2026_05_target_id_idx;


--
-- Name: audit_logs_2026_06_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2026_06_actor_id_idx;


--
-- Name: audit_logs_2026_06_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2026_06_created_at_idx;


--
-- Name: audit_logs_2026_06_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2026_06_event_type_idx;


--
-- Name: audit_logs_2026_06_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2026_06_guild_id_created_at_idx;


--
-- Name: audit_logs_2026_06_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2026_06_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2026_06_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2026_06_pkey;


--
-- Name: audit_logs_2026_06_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2026_06_target_id_idx;


--
-- Name: audit_logs_2026_07_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2026_07_actor_id_idx;


--
-- Name: audit_logs_2026_07_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2026_07_created_at_idx;


--
-- Name: audit_logs_2026_07_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2026_07_event_type_idx;


--
-- Name: audit_logs_2026_07_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2026_07_guild_id_created_at_idx;


--
-- Name: audit_logs_2026_07_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2026_07_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2026_07_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2026_07_pkey;


--
-- Name: audit_logs_2026_07_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2026_07_target_id_idx;


--
-- Name: audit_logs_2026_08_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2026_08_actor_id_idx;


--
-- Name: audit_logs_2026_08_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2026_08_created_at_idx;


--
-- Name: audit_logs_2026_08_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2026_08_event_type_idx;


--
-- Name: audit_logs_2026_08_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2026_08_guild_id_created_at_idx;


--
-- Name: audit_logs_2026_08_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2026_08_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2026_08_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2026_08_pkey;


--
-- Name: audit_logs_2026_08_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2026_08_target_id_idx;


--
-- Name: audit_logs_2026_09_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2026_09_actor_id_idx;


--
-- Name: audit_logs_2026_09_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2026_09_created_at_idx;


--
-- Name: audit_logs_2026_09_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2026_09_event_type_idx;


--
-- Name: audit_logs_2026_09_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2026_09_guild_id_created_at_idx;


--
-- Name: audit_logs_2026_09_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2026_09_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2026_09_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2026_09_pkey;


--
-- Name: audit_logs_2026_09_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2026_09_target_id_idx;


--
-- Name: audit_logs_2026_10_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2026_10_actor_id_idx;


--
-- Name: audit_logs_2026_10_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2026_10_created_at_idx;


--
-- Name: audit_logs_2026_10_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2026_10_event_type_idx;


--
-- Name: audit_logs_2026_10_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2026_10_guild_id_created_at_idx;


--
-- Name: audit_logs_2026_10_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2026_10_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2026_10_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2026_10_pkey;


--
-- Name: audit_logs_2026_10_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2026_10_target_id_idx;


--
-- Name: audit_logs_2026_11_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2026_11_actor_id_idx;


--
-- Name: audit_logs_2026_11_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2026_11_created_at_idx;


--
-- Name: audit_logs_2026_11_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2026_11_event_type_idx;


--
-- Name: audit_logs_2026_11_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2026_11_guild_id_created_at_idx;


--
-- Name: audit_logs_2026_11_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2026_11_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2026_11_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2026_11_pkey;


--
-- Name: audit_logs_2026_11_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2026_11_target_id_idx;


--
-- Name: audit_logs_2026_12_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2026_12_actor_id_idx;


--
-- Name: audit_logs_2026_12_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2026_12_created_at_idx;


--
-- Name: audit_logs_2026_12_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2026_12_event_type_idx;


--
-- Name: audit_logs_2026_12_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2026_12_guild_id_created_at_idx;


--
-- Name: audit_logs_2026_12_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2026_12_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2026_12_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2026_12_pkey;


--
-- Name: audit_logs_2026_12_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2026_12_target_id_idx;


--
-- Name: audit_logs_2027_01_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2027_01_actor_id_idx;


--
-- Name: audit_logs_2027_01_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2027_01_created_at_idx;


--
-- Name: audit_logs_2027_01_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2027_01_event_type_idx;


--
-- Name: audit_logs_2027_01_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2027_01_guild_id_created_at_idx;


--
-- Name: audit_logs_2027_01_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2027_01_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2027_01_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2027_01_pkey;


--
-- Name: audit_logs_2027_01_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2027_01_target_id_idx;


--
-- Name: audit_logs_2027_02_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2027_02_actor_id_idx;


--
-- Name: audit_logs_2027_02_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2027_02_created_at_idx;


--
-- Name: audit_logs_2027_02_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2027_02_event_type_idx;


--
-- Name: audit_logs_2027_02_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2027_02_guild_id_created_at_idx;


--
-- Name: audit_logs_2027_02_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2027_02_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2027_02_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2027_02_pkey;


--
-- Name: audit_logs_2027_02_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2027_02_target_id_idx;


--
-- Name: audit_logs_2027_03_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_2027_03_actor_id_idx;


--
-- Name: audit_logs_2027_03_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_2027_03_created_at_idx;


--
-- Name: audit_logs_2027_03_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_2027_03_event_type_idx;


--
-- Name: audit_logs_2027_03_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_2027_03_guild_id_created_at_idx;


--
-- Name: audit_logs_2027_03_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_2027_03_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_2027_03_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_2027_03_pkey;


--
-- Name: audit_logs_2027_03_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_2027_03_target_id_idx;


--
-- Name: audit_logs_default_actor_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_actor ATTACH PARTITION public.audit_logs_default_actor_id_idx;


--
-- Name: audit_logs_default_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_created_at ATTACH PARTITION public.audit_logs_default_created_at_idx;


--
-- Name: audit_logs_default_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_event_type ATTACH PARTITION public.audit_logs_default_event_type_idx;


--
-- Name: audit_logs_default_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_created ATTACH PARTITION public.audit_logs_default_guild_id_created_at_idx;


--
-- Name: audit_logs_default_guild_id_event_type_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_guild_type_date ATTACH PARTITION public.audit_logs_default_guild_id_event_type_created_at_idx;


--
-- Name: audit_logs_default_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.audit_logs_pkey1 ATTACH PARTITION public.audit_logs_default_pkey;


--
-- Name: audit_logs_default_target_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_audit_logs_target ATTACH PARTITION public.audit_logs_default_target_id_idx;


--
-- Name: infractions_2026_04_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2026_04_flags_idx;


--
-- Name: infractions_2026_04_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2026_04_guild_id_action_created_at_idx;


--
-- Name: infractions_2026_04_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2026_04_guild_id_action_idx;


--
-- Name: infractions_2026_04_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2026_04_guild_id_created_at_idx;


--
-- Name: infractions_2026_04_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2026_04_guild_id_user_id_idx;


--
-- Name: infractions_2026_04_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2026_04_pkey;


--
-- Name: infractions_2026_05_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2026_05_flags_idx;


--
-- Name: infractions_2026_05_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2026_05_guild_id_action_created_at_idx;


--
-- Name: infractions_2026_05_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2026_05_guild_id_action_idx;


--
-- Name: infractions_2026_05_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2026_05_guild_id_created_at_idx;


--
-- Name: infractions_2026_05_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2026_05_guild_id_user_id_idx;


--
-- Name: infractions_2026_05_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2026_05_pkey;


--
-- Name: infractions_2026_06_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2026_06_flags_idx;


--
-- Name: infractions_2026_06_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2026_06_guild_id_action_created_at_idx;


--
-- Name: infractions_2026_06_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2026_06_guild_id_action_idx;


--
-- Name: infractions_2026_06_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2026_06_guild_id_created_at_idx;


--
-- Name: infractions_2026_06_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2026_06_guild_id_user_id_idx;


--
-- Name: infractions_2026_06_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2026_06_pkey;


--
-- Name: infractions_2026_07_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2026_07_flags_idx;


--
-- Name: infractions_2026_07_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2026_07_guild_id_action_created_at_idx;


--
-- Name: infractions_2026_07_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2026_07_guild_id_action_idx;


--
-- Name: infractions_2026_07_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2026_07_guild_id_created_at_idx;


--
-- Name: infractions_2026_07_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2026_07_guild_id_user_id_idx;


--
-- Name: infractions_2026_07_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2026_07_pkey;


--
-- Name: infractions_2026_08_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2026_08_flags_idx;


--
-- Name: infractions_2026_08_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2026_08_guild_id_action_created_at_idx;


--
-- Name: infractions_2026_08_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2026_08_guild_id_action_idx;


--
-- Name: infractions_2026_08_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2026_08_guild_id_created_at_idx;


--
-- Name: infractions_2026_08_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2026_08_guild_id_user_id_idx;


--
-- Name: infractions_2026_08_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2026_08_pkey;


--
-- Name: infractions_2026_09_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2026_09_flags_idx;


--
-- Name: infractions_2026_09_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2026_09_guild_id_action_created_at_idx;


--
-- Name: infractions_2026_09_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2026_09_guild_id_action_idx;


--
-- Name: infractions_2026_09_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2026_09_guild_id_created_at_idx;


--
-- Name: infractions_2026_09_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2026_09_guild_id_user_id_idx;


--
-- Name: infractions_2026_09_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2026_09_pkey;


--
-- Name: infractions_2026_10_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2026_10_flags_idx;


--
-- Name: infractions_2026_10_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2026_10_guild_id_action_created_at_idx;


--
-- Name: infractions_2026_10_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2026_10_guild_id_action_idx;


--
-- Name: infractions_2026_10_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2026_10_guild_id_created_at_idx;


--
-- Name: infractions_2026_10_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2026_10_guild_id_user_id_idx;


--
-- Name: infractions_2026_10_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2026_10_pkey;


--
-- Name: infractions_2026_11_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2026_11_flags_idx;


--
-- Name: infractions_2026_11_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2026_11_guild_id_action_created_at_idx;


--
-- Name: infractions_2026_11_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2026_11_guild_id_action_idx;


--
-- Name: infractions_2026_11_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2026_11_guild_id_created_at_idx;


--
-- Name: infractions_2026_11_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2026_11_guild_id_user_id_idx;


--
-- Name: infractions_2026_11_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2026_11_pkey;


--
-- Name: infractions_2026_12_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2026_12_flags_idx;


--
-- Name: infractions_2026_12_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2026_12_guild_id_action_created_at_idx;


--
-- Name: infractions_2026_12_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2026_12_guild_id_action_idx;


--
-- Name: infractions_2026_12_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2026_12_guild_id_created_at_idx;


--
-- Name: infractions_2026_12_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2026_12_guild_id_user_id_idx;


--
-- Name: infractions_2026_12_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2026_12_pkey;


--
-- Name: infractions_2027_01_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2027_01_flags_idx;


--
-- Name: infractions_2027_01_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2027_01_guild_id_action_created_at_idx;


--
-- Name: infractions_2027_01_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2027_01_guild_id_action_idx;


--
-- Name: infractions_2027_01_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2027_01_guild_id_created_at_idx;


--
-- Name: infractions_2027_01_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2027_01_guild_id_user_id_idx;


--
-- Name: infractions_2027_01_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2027_01_pkey;


--
-- Name: infractions_2027_02_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2027_02_flags_idx;


--
-- Name: infractions_2027_02_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2027_02_guild_id_action_created_at_idx;


--
-- Name: infractions_2027_02_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2027_02_guild_id_action_idx;


--
-- Name: infractions_2027_02_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2027_02_guild_id_created_at_idx;


--
-- Name: infractions_2027_02_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2027_02_guild_id_user_id_idx;


--
-- Name: infractions_2027_02_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2027_02_pkey;


--
-- Name: infractions_2027_03_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_2027_03_flags_idx;


--
-- Name: infractions_2027_03_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_2027_03_guild_id_action_created_at_idx;


--
-- Name: infractions_2027_03_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_2027_03_guild_id_action_idx;


--
-- Name: infractions_2027_03_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_2027_03_guild_id_created_at_idx;


--
-- Name: infractions_2027_03_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_2027_03_guild_id_user_id_idx;


--
-- Name: infractions_2027_03_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_2027_03_pkey;


--
-- Name: infractions_default_flags_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_flags_gin ATTACH PARTITION public.infractions_default_flags_idx;


--
-- Name: infractions_default_guild_id_action_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action_created ATTACH PARTITION public.infractions_default_guild_id_action_created_at_idx;


--
-- Name: infractions_default_guild_id_action_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_action ATTACH PARTITION public.infractions_default_guild_id_action_idx;


--
-- Name: infractions_default_guild_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_guild_created ATTACH PARTITION public.infractions_default_guild_id_created_at_idx;


--
-- Name: infractions_default_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_infractions_user ATTACH PARTITION public.infractions_default_guild_id_user_id_idx;


--
-- Name: infractions_default_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.infractions_pkey1 ATTACH PARTITION public.infractions_default_pkey;


--
-- Name: logs_2026_04_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2026_04_bot_idx;


--
-- Name: logs_2026_04_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2026_04_level_idx;


--
-- Name: logs_2026_04_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2026_04_pkey;


--
-- Name: logs_2026_04_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2026_04_timestamp_idx;


--
-- Name: logs_2026_05_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2026_05_bot_idx;


--
-- Name: logs_2026_05_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2026_05_level_idx;


--
-- Name: logs_2026_05_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2026_05_pkey;


--
-- Name: logs_2026_05_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2026_05_timestamp_idx;


--
-- Name: logs_2026_06_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2026_06_bot_idx;


--
-- Name: logs_2026_06_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2026_06_level_idx;


--
-- Name: logs_2026_06_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2026_06_pkey;


--
-- Name: logs_2026_06_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2026_06_timestamp_idx;


--
-- Name: logs_2026_07_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2026_07_bot_idx;


--
-- Name: logs_2026_07_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2026_07_level_idx;


--
-- Name: logs_2026_07_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2026_07_pkey;


--
-- Name: logs_2026_07_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2026_07_timestamp_idx;


--
-- Name: logs_2026_08_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2026_08_bot_idx;


--
-- Name: logs_2026_08_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2026_08_level_idx;


--
-- Name: logs_2026_08_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2026_08_pkey;


--
-- Name: logs_2026_08_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2026_08_timestamp_idx;


--
-- Name: logs_2026_09_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2026_09_bot_idx;


--
-- Name: logs_2026_09_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2026_09_level_idx;


--
-- Name: logs_2026_09_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2026_09_pkey;


--
-- Name: logs_2026_09_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2026_09_timestamp_idx;


--
-- Name: logs_2026_10_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2026_10_bot_idx;


--
-- Name: logs_2026_10_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2026_10_level_idx;


--
-- Name: logs_2026_10_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2026_10_pkey;


--
-- Name: logs_2026_10_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2026_10_timestamp_idx;


--
-- Name: logs_2026_11_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2026_11_bot_idx;


--
-- Name: logs_2026_11_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2026_11_level_idx;


--
-- Name: logs_2026_11_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2026_11_pkey;


--
-- Name: logs_2026_11_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2026_11_timestamp_idx;


--
-- Name: logs_2026_12_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2026_12_bot_idx;


--
-- Name: logs_2026_12_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2026_12_level_idx;


--
-- Name: logs_2026_12_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2026_12_pkey;


--
-- Name: logs_2026_12_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2026_12_timestamp_idx;


--
-- Name: logs_2027_01_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2027_01_bot_idx;


--
-- Name: logs_2027_01_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2027_01_level_idx;


--
-- Name: logs_2027_01_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2027_01_pkey;


--
-- Name: logs_2027_01_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2027_01_timestamp_idx;


--
-- Name: logs_2027_02_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2027_02_bot_idx;


--
-- Name: logs_2027_02_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2027_02_level_idx;


--
-- Name: logs_2027_02_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2027_02_pkey;


--
-- Name: logs_2027_02_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2027_02_timestamp_idx;


--
-- Name: logs_2027_03_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_2027_03_bot_idx;


--
-- Name: logs_2027_03_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_2027_03_level_idx;


--
-- Name: logs_2027_03_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_2027_03_pkey;


--
-- Name: logs_2027_03_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_2027_03_timestamp_idx;


--
-- Name: logs_default_bot_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_bot ATTACH PARTITION public.logs_default_bot_idx;


--
-- Name: logs_default_level_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_level ATTACH PARTITION public.logs_default_level_idx;


--
-- Name: logs_default_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.logs_pkey1 ATTACH PARTITION public.logs_default_pkey;


--
-- Name: logs_default_timestamp_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_logs_timestamp ATTACH PARTITION public.logs_default_timestamp_idx;


--
-- Name: user_activity_log_2026_04_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2026_04_created_at_idx;


--
-- Name: user_activity_log_2026_04_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2026_04_event_type_idx;


--
-- Name: user_activity_log_2026_04_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2026_04_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2026_04_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2026_04_guild_id_user_id_idx;


--
-- Name: user_activity_log_2026_04_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2026_04_pkey;


--
-- Name: user_activity_log_2026_05_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2026_05_created_at_idx;


--
-- Name: user_activity_log_2026_05_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2026_05_event_type_idx;


--
-- Name: user_activity_log_2026_05_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2026_05_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2026_05_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2026_05_guild_id_user_id_idx;


--
-- Name: user_activity_log_2026_05_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2026_05_pkey;


--
-- Name: user_activity_log_2026_06_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2026_06_created_at_idx;


--
-- Name: user_activity_log_2026_06_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2026_06_event_type_idx;


--
-- Name: user_activity_log_2026_06_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2026_06_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2026_06_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2026_06_guild_id_user_id_idx;


--
-- Name: user_activity_log_2026_06_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2026_06_pkey;


--
-- Name: user_activity_log_2026_07_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2026_07_created_at_idx;


--
-- Name: user_activity_log_2026_07_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2026_07_event_type_idx;


--
-- Name: user_activity_log_2026_07_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2026_07_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2026_07_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2026_07_guild_id_user_id_idx;


--
-- Name: user_activity_log_2026_07_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2026_07_pkey;


--
-- Name: user_activity_log_2026_08_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2026_08_created_at_idx;


--
-- Name: user_activity_log_2026_08_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2026_08_event_type_idx;


--
-- Name: user_activity_log_2026_08_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2026_08_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2026_08_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2026_08_guild_id_user_id_idx;


--
-- Name: user_activity_log_2026_08_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2026_08_pkey;


--
-- Name: user_activity_log_2026_09_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2026_09_created_at_idx;


--
-- Name: user_activity_log_2026_09_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2026_09_event_type_idx;


--
-- Name: user_activity_log_2026_09_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2026_09_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2026_09_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2026_09_guild_id_user_id_idx;


--
-- Name: user_activity_log_2026_09_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2026_09_pkey;


--
-- Name: user_activity_log_2026_10_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2026_10_created_at_idx;


--
-- Name: user_activity_log_2026_10_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2026_10_event_type_idx;


--
-- Name: user_activity_log_2026_10_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2026_10_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2026_10_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2026_10_guild_id_user_id_idx;


--
-- Name: user_activity_log_2026_10_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2026_10_pkey;


--
-- Name: user_activity_log_2026_11_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2026_11_created_at_idx;


--
-- Name: user_activity_log_2026_11_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2026_11_event_type_idx;


--
-- Name: user_activity_log_2026_11_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2026_11_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2026_11_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2026_11_guild_id_user_id_idx;


--
-- Name: user_activity_log_2026_11_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2026_11_pkey;


--
-- Name: user_activity_log_2026_12_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2026_12_created_at_idx;


--
-- Name: user_activity_log_2026_12_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2026_12_event_type_idx;


--
-- Name: user_activity_log_2026_12_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2026_12_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2026_12_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2026_12_guild_id_user_id_idx;


--
-- Name: user_activity_log_2026_12_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2026_12_pkey;


--
-- Name: user_activity_log_2027_01_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2027_01_created_at_idx;


--
-- Name: user_activity_log_2027_01_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2027_01_event_type_idx;


--
-- Name: user_activity_log_2027_01_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2027_01_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2027_01_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2027_01_guild_id_user_id_idx;


--
-- Name: user_activity_log_2027_01_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2027_01_pkey;


--
-- Name: user_activity_log_2027_02_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2027_02_created_at_idx;


--
-- Name: user_activity_log_2027_02_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2027_02_event_type_idx;


--
-- Name: user_activity_log_2027_02_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2027_02_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2027_02_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2027_02_guild_id_user_id_idx;


--
-- Name: user_activity_log_2027_02_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2027_02_pkey;


--
-- Name: user_activity_log_2027_03_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_2027_03_created_at_idx;


--
-- Name: user_activity_log_2027_03_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_2027_03_event_type_idx;


--
-- Name: user_activity_log_2027_03_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_2027_03_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_2027_03_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_2027_03_guild_id_user_id_idx;


--
-- Name: user_activity_log_2027_03_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_2027_03_pkey;


--
-- Name: user_activity_log_default_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_created ATTACH PARTITION public.user_activity_log_default_created_at_idx;


--
-- Name: user_activity_log_default_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_type ATTACH PARTITION public.user_activity_log_default_event_type_idx;


--
-- Name: user_activity_log_default_guild_id_user_id_event_type_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user_type ATTACH PARTITION public.user_activity_log_default_guild_id_user_id_event_type_idx;


--
-- Name: user_activity_log_default_guild_id_user_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_user_activity_guild_user ATTACH PARTITION public.user_activity_log_default_guild_id_user_id_idx;


--
-- Name: user_activity_log_default_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.user_activity_log_pkey1 ATTACH PARTITION public.user_activity_log_default_pkey;


--
-- Name: announcement_button_interactions announcement_button_interactions_announcement_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.announcement_button_interactions
    ADD CONSTRAINT announcement_button_interactions_announcement_id_fkey FOREIGN KEY (announcement_id) REFERENCES public.scheduled_announcements(id) ON DELETE CASCADE;


--
-- Name: api_user_guilds api_user_guilds_discord_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.api_user_guilds
    ADD CONSTRAINT api_user_guilds_discord_user_id_fkey FOREIGN KEY (discord_user_id) REFERENCES public.api_users(discord_user_id) ON DELETE CASCADE;


--
-- Name: automod_discussion_channels automod_discussion_channels_review_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.automod_discussion_channels
    ADD CONSTRAINT automod_discussion_channels_review_id_fkey FOREIGN KEY (review_id) REFERENCES public.automod_reviews(id) ON DELETE CASCADE;


--
-- Name: automod_discussion_messages automod_discussion_messages_review_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.automod_discussion_messages
    ADD CONSTRAINT automod_discussion_messages_review_id_fkey FOREIGN KEY (review_id) REFERENCES public.automod_reviews(id) ON DELETE CASCADE;


--
-- Name: automod_review_votes automod_review_votes_review_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.automod_review_votes
    ADD CONSTRAINT automod_review_votes_review_id_fkey FOREIGN KEY (review_id) REFERENCES public.automod_reviews(id) ON DELETE CASCADE;


--
-- Name: confession_replies confession_replies_confession_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.confession_replies
    ADD CONSTRAINT confession_replies_confession_id_fkey FOREIGN KEY (confession_id) REFERENCES public.confessions(id) ON DELETE CASCADE;


--
-- Name: confession_reports confession_reports_confession_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.confession_reports
    ADD CONSTRAINT confession_reports_confession_id_fkey FOREIGN KEY (confession_id) REFERENCES public.confessions(id) ON DELETE CASCADE;


--
-- Name: confession_reports confession_reports_reply_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.confession_reports
    ADD CONSTRAINT confession_reports_reply_id_fkey FOREIGN KEY (reply_id) REFERENCES public.confession_replies(id) ON DELETE CASCADE;


--
-- Name: moderation_evidence moderation_evidence_action_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.moderation_evidence
    ADD CONSTRAINT moderation_evidence_action_id_fkey FOREIGN KEY (action_id) REFERENCES public.moderation_actions(id) ON DELETE CASCADE;


--
-- Name: review_queue review_queue_action_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.review_queue
    ADD CONSTRAINT review_queue_action_id_fkey FOREIGN KEY (action_id) REFERENCES public.moderation_actions(id) ON DELETE CASCADE;


--
-- Name: role_panel_entries role_panel_entries_panel_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.role_panel_entries
    ADD CONSTRAINT role_panel_entries_panel_id_fkey FOREIGN KEY (panel_id) REFERENCES public.role_panels(id) ON DELETE CASCADE;


--
-- Name: scheduled_announcement_runs scheduled_announcement_runs_announcement_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.scheduled_announcement_runs
    ADD CONSTRAINT scheduled_announcement_runs_announcement_id_fkey FOREIGN KEY (announcement_id) REFERENCES public.scheduled_announcements(id) ON DELETE CASCADE;


--
-- Name: ticket_assignments ticket_assignments_ticket_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ticket_assignments
    ADD CONSTRAINT ticket_assignments_ticket_id_fkey FOREIGN KEY (ticket_id) REFERENCES public.tickets(id) ON DELETE CASCADE;


--
-- Name: ticket_messages ticket_messages_ticket_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ticket_messages
    ADD CONSTRAINT ticket_messages_ticket_id_fkey FOREIGN KEY (ticket_id) REFERENCES public.tickets(id) ON DELETE CASCADE;


--
-- Name: voice_channel_co_admins voice_channel_co_admins_voice_channel_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_co_admins
    ADD CONSTRAINT voice_channel_co_admins_voice_channel_id_fkey FOREIGN KEY (voice_channel_id) REFERENCES public.voice_channels(id) ON DELETE CASCADE;


--
-- Name: voice_channel_invite_links voice_channel_invite_links_voice_channel_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channel_invite_links
    ADD CONSTRAINT voice_channel_invite_links_voice_channel_id_fkey FOREIGN KEY (voice_channel_id) REFERENCES public.voice_channels(id) ON DELETE CASCADE;


--
--


--
-- Seeds
--

--
--


--
-- Data for Name: alert_rules; Type: TABLE DATA; Schema: public; Owner: sentinel_test
--

INSERT INTO public.alert_rules VALUES ('cpu_percent', 'CPU host eleve', 'cpu_percent', 'gt', 90, true, 'warning', 1800, '2026-07-28 14:21:51.343663+00');
INSERT INTO public.alert_rules VALUES ('mem_percent', 'RAM host elevee', 'mem_percent', 'gt', 90, true, 'warning', 1800, '2026-07-28 14:21:51.343663+00');
INSERT INTO public.alert_rules VALUES ('disk_percent', 'Disque presque plein', 'disk_percent', 'gt', 85, true, 'critical', 3600, '2026-07-28 14:21:51.343663+00');
INSERT INTO public.alert_rules VALUES ('auth_failures_1h', 'Echecs d''auth (brute-force)', 'auth_failures_1h', 'gt', 50, true, 'critical', 3600, '2026-07-28 14:21:51.343663+00');
INSERT INTO public.alert_rules VALUES ('service_offline', 'Service bot/worker offline', 'service_offline', 'gt', NULL, true, 'critical', 1800, '2026-07-28 14:21:51.343663+00');
INSERT INTO public.alert_rules VALUES ('tls_expiry_days', 'Certificat TLS bientot expire', 'tls_expiry_days', 'lt', 14, true, 'warning', 86400, '2026-07-28 14:21:51.343663+00');
INSERT INTO public.alert_rules VALUES ('container_removed', 'Conteneur supprime/modifie', 'container_removed', 'gt', NULL, true, 'warning', 3600, '2026-07-28 14:21:51.343663+00');


--
-- Data for Name: bot_definitions; Type: TABLE DATA; Schema: public; Owner: sentinel_test
--

INSERT INTO public.bot_definitions VALUES ('automod-bot', 'AutoMod', 'Moderation automatique des messages (spam, insultes, liens)', '[{"key": "enabled", "type": "boolean", "label": "Active", "default": "true", "required": false, "description": "Active ou desactive completement l auto-moderation. Si OFF, aucun message n est analyse."}, {"key": "spam_detection_enabled", "type": "boolean", "label": "Detection spam", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Detecte les messages dupliques / floodes (regex)."}, {"key": "spam_repeat_char_threshold", "type": "number", "label": "Seuil caracteres repetes", "default": "6", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre minimum de caracteres identiques consecutifs pour declencher la detection (ex: aaaaaa)."}, {"key": "spam_repeat_word_threshold", "type": "number", "label": "Seuil mots repetes", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre minimum de mots identiques pour declencher la detection spam."}, {"key": "caps_warning_enabled", "type": "boolean", "label": "Avertissement majuscules", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Envoie un avertissement quand un message est ecrit entierement en MAJUSCULES."}, {"key": "caps_threshold_chars", "max": 500, "min": 5, "type": "number", "unit": "caracteres", "label": "Seuil majuscules", "default": "8", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "A partir de combien de caracteres en majuscules le message est flag."}, {"key": "insult_detection_enabled", "type": "boolean", "label": "Detection insultes", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Detecte les insultes par regex (FR + EN, leet speak). Complementaire de l IA texte qui detecte plutot les sentiments (rage, menace)."}, {"key": "insult_custom_words", "type": "text", "label": "Mots personnalises", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Liste de mots customs separes par des virgules. Detection case-insensitive, substring match."}, {"key": "link_detection_enabled", "type": "boolean", "label": "Detection liens", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Detecte les liens HTTP. Combine avec phishing_detection pour scoring eleve."}, {"key": "allow_discord_invites", "type": "boolean", "label": "Autoriser invitations Discord", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si desactive, les liens discord.gg seront supprimes."}, {"key": "allowed_domains", "type": "text", "label": "Domaines autorises", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Domaines autorises (separes par virgules). Ex: youtube.com,twitch.tv,twitter.com"}, {"key": "phishing_detection_enabled", "type": "boolean", "label": "Detection phishing", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Detecte les domaines de phishing connus. Score eleve par defaut (7.0) -> action severe."}, {"key": "phishing_extra_whitelist", "type": "text", "label": "Whitelist phishing", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Domaines supplementaires a ne pas considerer comme phishing (virgules)."}, {"key": "emoji_spam_enabled", "type": "boolean", "label": "Detection emoji spam", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Detecte les messages contenant un nombre excessif d emojis."}, {"key": "emoji_spam_max", "type": "number", "label": "Max emojis", "default": "10", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre maximum d emojis autorises par message avant detection."}, {"key": "mentions_enabled", "type": "boolean", "label": "Detection mentions", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Detecte les messages avec trop de mentions (@user)."}, {"key": "mentions_max", "type": "number", "label": "Max mentions", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre maximum de mentions par message avant detection."}, {"key": "suspicious_files_enabled", "type": "boolean", "label": "Detection fichiers suspects", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Detecte les pieces jointes a extension dangereuse (.exe, .bat, .vbs, etc.)."}, {"key": "suspicious_file_extensions", "type": "text", "label": "Extensions suspectes", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Extensions supplementaires a bloquer (virgules). Ex: apk,iso,torrent"}, {"key": "unicode_detection_enabled", "type": "boolean", "label": "Detection unicode", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Detecte les caracteres Unicode invisibles ou de combinaison excessifs."}, {"key": "unicode_max_combining", "type": "number", "label": "Max combinaisons Unicode", "default": "3", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre maximum de caracteres combinants Unicode par caractere."}, {"key": "unicode_max_invisible", "type": "number", "label": "Max invisibles Unicode", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre maximum de caracteres invisibles (zero-width) par message."}, {"key": "flood_max_messages", "max": 100, "min": 2, "type": "number", "unit": "messages", "label": "Seuil flood (messages)", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre max de messages dans la fenetre flood avant warning + envoi a l IA."}, {"key": "flood_window_secs", "max": 300, "min": 1, "type": "number", "unit": "secondes", "label": "Fenetre flood (secondes)", "default": "10", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Fenetre temporelle pour le flood. Defaut : 10s pour 5 messages."}, {"key": "mute_duration_secs", "max": 2419200, "min": 60, "type": "number", "unit": "secondes", "label": "Duree mute (secondes)", "default": "600", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Duree du timeout Discord applique en cas d action mute. Max Discord : 28 jours."}, {"key": "night_mode_enabled", "type": "boolean", "label": "Mode nuit", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active des seuils plus stricts pendant les heures de nuit (ci-dessous)."}, {"key": "night_start_hour", "max": 23, "min": 0, "type": "number", "unit": "heure", "label": "Heure debut nuit", "default": "22", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Heure de debut du night mode (24h)."}, {"key": "night_end_hour", "max": 23, "min": 0, "type": "number", "unit": "heure", "label": "Heure fin nuit", "default": "8", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Heure de fin du night mode (24h)."}, {"key": "adaptive_slowmode_enabled", "type": "boolean", "label": "Slowmode adaptatif", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active automatiquement le slowmode Discord si le salon depasse un seuil de messages."}, {"key": "adaptive_slowmode_threshold", "max": 200, "min": 1, "type": "number", "unit": "messages/min", "label": "Seuil slowmode", "default": "15", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Seuil de messages par minute pour declencher le slowmode."}, {"key": "adaptive_slowmode_seconds", "max": 21600, "min": 1, "type": "number", "unit": "secondes", "label": "Duree slowmode", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Duree du slowmode applique."}, {"key": "log_channel_id", "type": "channel", "label": "Salon de logs", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon ou les actions automod sont loggees (suppressions, mutes, cartes de review). Indispensable si ai_review_mode = true."}, {"key": "ignored_channels", "type": "channel_list", "label": "Salons ignores", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "IDs de salons exclus de l automod, separes par virgules."}, {"key": "ignored_roles", "type": "role_list", "label": "Roles ignores", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "IDs de roles dont les membres sont exclus de l automod (mods, etc.), separes par virgules."}, {"key": "color_warn", "type": "text", "label": "Couleur avertissement", "default": "f59e0b", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Code couleur hex pour les embeds d avertissement."}, {"key": "color_delete", "type": "text", "label": "Couleur suppression", "default": "f97316", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Code couleur hex pour les embeds de suppression."}, {"key": "color_mute", "type": "text", "label": "Couleur mute", "default": "ef4444", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Code couleur hex pour les embeds de mute."}, {"key": "color_ban", "type": "text", "label": "Couleur ban", "default": "dc2626", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Code couleur hex pour les embeds de bannissement."}, {"key": "context_max_messages", "max": 20, "min": 0, "type": "number", "unit": "messages", "label": "Messages de contexte (nombre)", "default": "3", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre de messages precedents inclus comme contexte. 0 = pas de contexte."}, {"key": "context_max_chars", "max": 1000, "min": 50, "type": "number", "unit": "caracteres", "label": "Caracteres max par message de contexte", "default": "200", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Longueur max de chaque message de contexte. Au-dela, tronque."}, {"key": "ai_review_mode", "type": "boolean", "label": "Mode review IA...", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si ON, le bot envoie une carte de review au mod au lieu d agir directement. Tres utile en phase de tuning. Si OFF, l action est appliquee automatiquement."}, {"key": "flood_review_mode", "type": "boolean", "label": "Mode review flood", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si ON, le flood passe en carte de review au lieu de warner directement."}, {"key": "caps_review_mode", "type": "boolean", "label": "Mode review majuscules", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si ON, l abus de majuscules passe en carte de review."}, {"key": "files_review_mode", "type": "boolean", "label": "Mode review fichiers suspects", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si ON, les fichiers suspects sont mis en review (pas supprimes auto)."}, {"key": "text_enabled", "type": "boolean", "label": "Analyse IA texte activee", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active l inference IA texte (DistilBERT) pour detecter rage / menace / harcelement / colere."}, {"key": "vision_enabled", "type": "boolean", "label": "Analyse IA images activee", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active l inference IA vision (EfficientNet) sur les images jointes. Detecte NSFW et contenu illicite."}, {"key": "channel_tension_enabled", "type": "boolean", "label": "Tension de salon activee", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active la detection d escalade par somme glissante des scores IA sur les N derniers messages d un salon."}, {"key": "channel_tension_buffer_size", "type": "number", "label": "Taille du buffer glissant", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre de derniers messages d un salon inclus dans le calcul de tension."}, {"key": "channel_tension_threshold_warn", "type": "number", "label": "Seuil tension - Warn", "default": "3.0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Somme cumulee des scores IA a partir de laquelle un warning est emis (0 pour desactiver ce palier)."}, {"key": "channel_tension_threshold_delete", "type": "number", "label": "Seuil tension - Delete", "default": "5.0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Somme cumulee des scores IA a partir de laquelle le dernier message est supprime (0 pour desactiver)."}, {"key": "channel_tension_threshold_mute", "type": "number", "label": "Seuil tension - Mute", "default": "7.0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Somme cumulee des scores IA a partir de laquelle le dernier auteur est mute (0 pour desactiver)."}, {"key": "channel_tension_mute_duration_secs", "type": "number", "label": "Duree du mute tension (secondes)", "default": "300", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Duree du mute declenche par la tension de salon."}, {"key": "channel_tension_warning_channel_id", "type": "channel", "label": "Salon de notification tension", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon ou poster les alertes de tension. Si vide, le message est poste dans le salon courant."}, {"key": "text_threshold", "max": 1, "min": 0, "type": "number", "unit": "0..1", "label": "Seuil confidence IA texte", "default": "0.5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Confidence minimale pour qu un flag IA soit actif. Plus bas = plus sensible. Recommande : 0.5. A baisser a 0.35 pour catcher des cas borderline."}, {"key": "vision_threshold", "max": 1, "min": 0, "type": "number", "unit": "0..1", "label": "Seuil confidence IA vision", "default": "0.5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Confidence minimale pour qu un flag vision soit actif. Recommande : 0.5."}, {"key": "context_dampening", "max": 1, "min": 0, "type": "number", "unit": "0..1", "label": "Attenuation contexte conversationnel", "default": "0.65", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Multiplicateur du score IA si du contexte conversationnel est present. 1.0 = pas d attenuation, 0.65 = score divise par 1.5 (defaut). Reduit les faux positifs entre potes."}, {"key": "context_format", "type": "enum", "label": "Format contexte IA", "default": "natural", "options": [{"label": "Naturel (texte brut)", "value": "natural"}, {"label": "Balises [message]/[context]", "value": "tagged"}], "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Comment le contexte est formate pour l IA. natural = simple, tagged = balises explicites (peut ameliorer la qualite selon le modele)."}, {"key": "vision_channel_thresholds", "type": "text", "label": "Seuils vision par salon", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Override du vision_threshold par salon. Format CSV : channel_id:threshold,channel_id:threshold."}, {"key": "vision_hash_cache_enabled", "type": "boolean", "label": "Cache hash images", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active le cache des hash d images analysees pour eviter de relancer l IA sur la meme image."}, {"key": "vision_hash_cache_ttl_secs", "max": 2592000, "min": 60, "type": "number", "unit": "secondes", "label": "TTL cache hash images", "default": "86400", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Duree de validite d un hash en cache. Recommande : 86400 (1 jour)."}, {"key": "vision_max_image_size_mb", "max": 25, "min": 1, "type": "number", "unit": "Mo", "label": "Taille max images", "default": "10", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Taille max d une image analysee. Au-dela, skip. Discord upload limite a 25Mo."}, {"key": "vision_queue_enabled", "type": "boolean", "label": "File async (ai-worker)", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si ON, l analyse d image est asynchrone via ai-worker (POST /api/ai/jobs). Sinon, synchrone bloquant."}, {"key": "vision_queue_max_retries", "max": 10, "min": 0, "type": "number", "unit": "tentatives", "label": "Tentatives max queue", "default": "3", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre max de retries sur un job IA en echec avant abandon."}, {"key": "vision_scan_embeds", "type": "boolean", "label": "Analyser images dans embeds", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Analyse aussi les images presentes dans les embeds (liens preview), pas seulement les pieces jointes."}, {"key": "vision_auto_delete_nsfw", "type": "boolean", "label": "Suppression auto NSFW", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Supprime automatiquement les images detectees NSFW au-dessus du seuil. Si OFF, le scoring decide via les seuils warn/delete/mute/ban."}, {"key": "vision_auto_delete_illicit", "type": "boolean", "label": "Suppression auto illicite", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Supprime automatiquement les images detectees comme contenu illicite. Recommande ON (poids defaut 9.0 deja eleve)."}, {"key": "review_min_score", "max": 10.0, "min": 0.0, "type": "number", "label": "Score IA minimum pour declencher une review", "default": "0.0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "En dessous de ce score, le bot applique l action automatiquement sans poster de carte de review. Au dessus, il poste une carte que la web peut resoudre. 0.0 = toutes les detections passent par la review."}, {"key": "vote_enabled", "type": "boolean", "label": "Vote des moderateurs", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si ON, une detection automod ouvre un VOTE des moderateurs (choix de la sanction) au lieu d appliquer directement. L admin finalise ensuite."}, {"key": "vote_deadline_hours", "max": 720, "min": 1, "type": "number", "unit": "heures", "label": "Delai de vote", "default": "72", "required": false, "depends_on": {"key": "vote_enabled", "equals": "true"}, "description": "Duree pendant laquelle les moderateurs peuvent voter. A l echeance, on compte les votes exprimes."}, {"key": "vote_quorum", "max": 50, "min": 1, "type": "number", "unit": "votes", "label": "Quorum minimum", "default": "3", "required": false, "depends_on": {"key": "vote_enabled", "equals": "true"}, "description": "Nombre minimum de votes exprimes pour que le verdict soit valable. En dessous, l alerte est ignoree."}, {"key": "vote_mod_role_id", "type": "role", "label": "Role autorise a voter", "required": false, "depends_on": {"key": "vote_enabled", "equals": "true"}, "description": "Role dont les membres peuvent voter. Vide = toute personne avec la permission Discord Moderer les membres."}, {"key": "vote_admin_role_id", "type": "role", "label": "Role autorise a finaliser", "required": false, "depends_on": {"key": "vote_enabled", "equals": "true"}, "description": "Role dont les membres peuvent appliquer/clore le verdict via le bouton admin. Vide = permission Administrateur."}, {"key": "vote_tie_action", "type": "enum", "label": "En cas d egalite", "default": "ignore", "options": [{"label": "Ignorer (aucune sanction)", "value": "ignore"}, {"label": "Sanction la plus clemente", "value": "clemente"}, {"label": "Sanction la plus severe", "value": "severe"}], "required": false, "depends_on": {"key": "vote_enabled", "equals": "true"}, "description": "Que faire quand deux sanctions sont a egalite de voix."}, {"key": "vote_context_before", "max": 25, "min": 0, "type": "number", "unit": "messages", "label": "Messages de contexte (avant)", "default": "10", "required": false, "depends_on": {"key": "vote_enabled", "equals": "true"}, "description": "Nombre de messages precedant le message signale, affiches sur la carte de vote pour donner du contexte. 0 = desactive."}, {"key": "vote_thread_enabled", "type": "boolean", "label": "Fil de discussion sur la carte", "default": "true", "required": false, "depends_on": {"key": "vote_enabled", "equals": "true"}, "description": "Ouvre automatiquement un fil de discussion attache a chaque carte de vote pour que les moderateurs en debattent."}, {"key": "vote_aggregate_enabled", "type": "boolean", "label": "Regrouper les alertes par utilisateur (1 carte/personne)", "default": "false", "required": false, "description": "Si ON, tant qu''une carte de vote est ouverte pour un membre, les nouveaux signalements s''y ajoutent (liste d''incidents + score cumule + deadline prolongee) au lieu de creer une nouvelle carte. Evite le flood de cartes quand un membre derape en serie."}, {"key": "discussion_channel_enabled", "type": "boolean", "label": "Bouton ''Ouvrir une discussion'' sur les cartes", "default": "false", "required": false, "description": "Si ON, chaque carte de vote affiche un bouton qui cree un salon textuel prive (membre concerne + role moderateur) avec un message de contexte epingle, pour discuter avant decision."}, {"key": "human_only_enabled", "type": "boolean", "label": "Modération 100% humaine (aucune sanction auto)", "default": "false", "required": false, "description": "Si ON, aucune sanction n''est appliquee automatiquement : chaque detection genere une carte que les moderateurs traitent (vote + finalisation). Necessite un salon de review configure."}, {"key": "dashboard_base_url", "type": "text", "label": "URL du dashboard (lien depuis les cartes)", "required": false, "description": "Base URL du dashboard web (ex: https://dash.exemple.com). Sert a generer le bouton \"Voir le detail\" sur les cartes de review/vote. Vide = pas de bouton."}, {"key": "auto_protect_enabled", "type": "boolean", "label": "Auto-protection des cas severes (raid / phishing / pub / gros flood)", "default": "true", "required": false, "description": "Si ON, les cas severes (phishing, invitation Discord, gros flood) declenchent une mesure reversible immediate (mute + suppression) MEME en moderation 100% humaine, puis une carte de review est toujours postee pour validation/ajustement par un moderateur."}, {"key": "severe_flood_max_messages", "type": "number", "label": "Seuil gros flood (messages) pour auto-protection", "default": "12", "required": false, "description": "Nombre de messages dans la fenetre de flood au-dela duquel on considere un gros flood / raid et on declenche l''auto-protection. Doit etre >= au seuil de flood simple."}, {"key": "auto_protect_notify_member", "type": "boolean", "label": "Informer le membre en DM (motif + droit d''appel)", "default": "true", "required": false, "description": "Si ON, lorsqu''une protection automatique (mute) est appliquee, le membre recoit un message prive avec le motif et la possibilite de contester via /appeal (conformite DSA)."}, {"key": "sanction_appeal_enabled", "type": "boolean", "label": "Mention du droit d''appel sur les messages de sanction", "default": "true", "required": false, "description": "Si ON, chaque message de sanction adresse au membre rappelle qu''il peut contester la decision via la commande /appeal (conformite DSA). Desactiver si le module d''appel n''est pas utilise."}, {"key": "auto_delete_links_enabled", "type": "boolean", "label": "Supprimer SÈCHEMENT les liens génériques (sinon : carte)", "default": "false", "required": false, "description": "OFF (défaut) : un lien générique non autorisé hors image génère une carte de review (décision humaine). ON : suppression automatique immédiate sans carte (mode agressif). Le phishing et les invitations Discord restent traités en auto-protection quoi qu''il arrive."}, {"key": "vote_aggregate_window_minutes", "type": "number", "label": "Fenêtre d''agrégation (minutes d''inactivité)", "default": "60", "required": false, "description": "Une carte agrégée cesse de se mettre à jour après ce délai sans nouvelle infraction. Une infraction ultérieure ouvre une nouvelle carte. Défaut : 60 minutes."}, {"key": "vote_context_after", "type": "number", "label": "Messages de contexte APRÈS l''infraction (salon de discussion)", "default": "10", "required": false, "description": "Nombre de messages postés APRÈS la dernière infraction à afficher dans le message d''ancrage du salon de discussion (0 = aucun)."}, {"key": "discussion_category_id", "type": "category", "label": "Categorie des salons de discussion", "required": false, "description": "Categorie sous laquelle creer les salons de discussion. Vide = a la racine du serveur."}, {"key": "automod_close_votes_secs", "max": 600, "min": 10, "type": "number", "unit": "s", "label": "Worker : intervalle cloture des votes", "default": "60", "required": false, "description": "Frequence a laquelle le worker ferme les cartes de vote de moderation arrivees a echeance. CRITIQUE : seule voie qui cloture les votes a leur deadline. Une valeur trop haute retarde la resolution des sanctions."}, {"key": "automod_cleanup_cards_secs", "max": 604800, "min": 3600, "type": "number", "unit": "s", "label": "Worker : intervalle nettoyage cartes closes", "default": "86400", "required": false, "description": "Frequence de purge des cartes de moderation closes depuis plus d un mois. La review et le transcript restent en base (trace web conservee)."}, {"key": "score_weight_spam", "min": 0, "type": "number", "label": "Scoring — poids spam", "default": "3", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand un message est detecte comme spam."}, {"key": "score_weight_insult", "min": 0, "type": "number", "label": "Scoring — poids insulte", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand un message contient une insulte."}, {"key": "score_weight_link", "min": 0, "type": "number", "label": "Scoring — poids lien", "default": "1", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand un message contient un lien."}, {"key": "score_weight_phishing", "min": 0, "type": "number", "label": "Scoring — poids phishing", "default": "7", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand un message est detecte comme phishing."}, {"key": "score_weight_nsfw", "min": 0, "type": "number", "label": "Scoring — poids NSFW (image)", "default": "8", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand une image est classee NSFW par la vision IA."}, {"key": "score_weight_illicit", "min": 0, "type": "number", "label": "Scoring — poids illicite (image)", "default": "9", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids ajoute au score quand une image est classee illicite par la vision IA."}, {"key": "score_weight_anger", "min": 0, "type": "number", "label": "Scoring — poids colere (IA texte)", "default": "3", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids de base du sentiment colere detecte par l IA texte (pondere par la confiance)."}, {"key": "score_weight_rage", "min": 0, "type": "number", "label": "Scoring — poids rage (IA texte)", "default": "6", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids de base du sentiment rage detecte par l IA texte (pondere par la confiance)."}, {"key": "score_weight_threat", "min": 0, "type": "number", "label": "Scoring — poids menace (IA texte)", "default": "8", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids de base du sentiment menace detecte par l IA texte (pondere par la confiance)."}, {"key": "score_weight_harassment", "min": 0, "type": "number", "label": "Scoring — poids harcelement (IA texte)", "default": "7", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Poids de base du sentiment harcelement detecte par l IA texte (pondere par la confiance)."}, {"key": "score_threshold_warn", "min": 0, "type": "number", "label": "Scoring — seuil warn", "default": "2", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Score total a partir duquel un avertissement est emis (baseline, si aucune regle per-flag ne s applique)."}, {"key": "score_threshold_delete", "min": 0, "type": "number", "label": "Scoring — seuil suppression", "default": "4", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Score total a partir duquel le message est supprime (baseline)."}, {"key": "score_threshold_mute", "min": 0, "type": "number", "label": "Scoring — seuil mute", "default": "6", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Score total a partir duquel l auteur est mute (baseline)."}, {"key": "score_threshold_ban", "min": 0, "type": "number", "label": "Scoring — seuil ban", "default": "9", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Score total a partir duquel l auteur est banni automatiquement (baseline)."}]');
INSERT INTO public.bot_definitions VALUES ('cleanup', 'Nettoyage automatique', 'Suppression periodique des donnees historiques (sessions vocales, logs, tickets fermes) + VACUUM Postgres pour optimiser la taille des tables. Les commandes /purge et /cleanup cote Discord sont aussi gerees ici.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Active la suppression automatique + les commandes /purge et /cleanup."}, {"key": "voice_sessions_retention_days", "max": 365, "min": 7, "type": "number", "unit": "jours", "label": "Retention sessions vocales", "default": "90", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Sessions plus anciennes que ce delai sont supprimees."}, {"key": "logs_retention_days", "max": 365, "min": 7, "type": "number", "unit": "jours", "label": "Retention logs", "default": "30", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "closed_tickets_retention_days", "max": 365, "min": 7, "type": "number", "unit": "jours", "label": "Retention tickets fermes", "default": "180", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "cleanup_interval_hours", "max": 168, "min": 1, "type": "number", "unit": "h", "label": "Intervalle nettoyage", "default": "1", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence du job de nettoyage. 1h = scan toutes les heures."}, {"key": "vacuum_enabled", "type": "boolean", "label": "VACUUM automatique", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Reclamation de l espace disque + maintenance des index Postgres."}, {"key": "vacuum_interval_hours", "max": 168, "min": 1, "type": "number", "unit": "h", "label": "Intervalle VACUUM", "default": "24", "required": false, "depends_on": {"key": "vacuum_enabled", "equals": "true"}, "description": "Frequence du VACUUM ANALYZE. 24h est generalement un bon compromis."}]');
INSERT INTO public.bot_definitions VALUES ('security-bot', 'Securite', 'Detection de raids et comptes suspects', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Active ou desactive le module security."}, {"key": "min_account_age_secs", "max": 31536000, "min": 0, "type": "number", "unit": "secondes", "label": "Age minimum du compte (secondes)", "default": "86400", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Age minimum d un compte Discord pour etre autorise a join sans suspicion. Recommande : 604800 (7 jours)."}, {"key": "quarantine_enabled", "type": "boolean", "label": "Quarantaine activee", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active le systeme de quarantaine (role @Quarantaine pour comptes suspects)."}, {"key": "quarantine_role_id", "type": "role", "label": "Role de quarantaine", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Role de quarantaine (acces ultra-restreint) pour comptes suspects."}, {"key": "captcha_enabled", "type": "boolean", "label": "Captcha active", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Force les nouveaux membres a passer un captcha avant d acceder au serveur."}, {"key": "slowmode_seconds", "max": 21600, "min": 0, "type": "number", "unit": "secondes", "label": "Slowmode anti-raid (secondes, 0 = desactive)", "default": "0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Slowmode applique automatiquement sur les salons concernes en cas de raid detecte. 0 = pas de slowmode."}, {"key": "lockdown_enabled", "type": "boolean", "label": "Lockdown auto (desactive envoi messages)", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Permet d activer le mode lockdown (nouveaux membres mutes auto) via /security lockdown."}, {"key": "captcha_type", "type": "enum", "label": "Type de captcha (button, math)", "default": "button", "options": [{"label": "Calcul mental", "value": "math"}, {"label": "Image avec bruit", "value": "image"}, {"label": "Simple bouton de verification", "value": "button"}], "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Type de captcha pour les nouveaux membres. button = simplest, image = anti-bot le plus efficace."}, {"key": "alt_detection_enabled", "type": "boolean", "label": "Detection de comptes alt", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Detecte les comptes alternatifs (meme IP, fingerprint similaire)."}, {"key": "raid_pattern_enabled", "type": "boolean", "label": "Detection patterns de raid avancee", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active la detection de patterns de raid avances (pseudos similaires, avatars identiques, dates de creation tres proches)."}, {"key": "raid_pattern_score_threshold", "max": 100, "min": 1, "type": "number", "unit": "score", "label": "Score seuil pattern raid (0-100)", "default": "60", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Score min de pattern raid pour declencher une alerte. Plus bas = plus sensible. Recommande : 50."}, {"key": "raid_mode", "type": "enum", "label": "Mode de réponse anti-raid", "default": "hybrid", "options": ["auto", "suggest", "hybrid"], "required": false, "description": "Mode de réponse anti-raid : auto (applique directement), suggest (demande confirmation staff), hybrid (auto si raid massif, sinon suggestion)."}, {"key": "raid_auto_threshold", "max": 100, "min": 0, "type": "number", "label": "Anti-raid — seuil auto", "default": "85", "required": false, "description": "En mode hybride : score de raid à partir duquel la réponse (lockdown/slowmode) est appliquée automatiquement ; en dessous elle est seulement suggérée au staff."}, {"key": "raid_suggest_channel_id", "type": "channel", "label": "Salon d''alerte anti-raid", "default": "", "required": false, "description": "Salon d''alerte anti-raid (suggestions). Vide -> repli sur le salon de logs sécurité, sinon application auto (protection avant silence)."}]');
INSERT INTO public.bot_definitions VALUES ('export', 'Export de donnees', 'Traite les demandes d export RGPD / sauvegarde de donnees Discord. Les requetes web sont mises en file export_jobs, le worker les depile et genere un fichier ZIP downloadable.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Si OFF : les demandes d export restent en attente."}, {"key": "export_scan_interval", "max": 300, "min": 1, "type": "number", "unit": "s", "label": "Intervalle depilage file export", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "max_rows_per_export", "max": 50000, "min": 1, "type": "number", "unit": "lignes", "label": "Worker : max lignes par export", "default": "50000", "required": false, "description": "Garde-fou memoire : nombre max de lignes retournees par un export. Au-dela l API tronque. 50k lignes JSON ~ 20-50 MB."}, {"key": "export_processing_timeout_secs", "max": 86400, "min": 30, "type": "number", "unit": "s", "label": "Worker : timeout job export zombie", "default": "300", "required": false, "description": "Duree au-dela de laquelle un export bloque en processing est considere zombie (worker crash) et remis en pending pour retry."}]');
INSERT INTO public.bot_definitions VALUES ('temp_roles', 'Roles temporaires', 'Retire automatiquement les roles temporaires expires (assignes via /role temp ou par d''autres modules). Sans ce module, les roles temporaires ne sont jamais retires.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Si OFF, les roles temporaires expires ne seront plus retires automatiquement."}, {"key": "temp_roles_scan_interval", "max": 3600, "min": 10, "type": "number", "unit": "s", "label": "Intervalle scan roles expires", "default": "60", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence de scan des roles temporaires a retirer. 60s = bon compromis precision/charge."}]');
INSERT INTO public.bot_definitions VALUES ('moderation-bot', 'Moderation', 'Sanctions manuelles (warn / mute / ban / kick), templates de raisons, appels, escalation automatique selon historique, regeneration des points de conduite, nettoyage des bans expires. Les jobs periodiques tournent dans sentinel-worker.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Active toutes les fonctionnalites moderation (commandes /ban, /mute, /warn, /kick et jobs worker)."}, {"key": "dm_on_sanction", "type": "boolean", "label": "DM lors de sanction", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Envoie un DM au membre sanctionne avec le motif et la duree."}, {"key": "templates_enabled", "type": "boolean", "label": "Templates de raisons", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active les templates de raisons rapides (/template apply)."}, {"key": "review_required_for", "type": "text", "label": "Actions en review obligatoire", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Liste CSV des actions necessitant review avant application (ex: ban,mute,kick)."}, {"key": "auto_archive_appeals_days", "max": 365, "min": 1, "type": "number", "unit": "j", "label": "Archivage auto des appels", "default": "30", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Apres combien de jours un appel non resolu est archive automatiquement."}, {"key": "default_warn_gravity", "type": "enum", "label": "Gravite par defaut (warn)", "default": "medium", "options": [{"label": "Faible", "value": "low"}, {"label": "Moyenne", "value": "medium"}, {"label": "Haute", "value": "high"}], "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Gravite par defaut quand un mod fait /warn sans la specifier."}, {"key": "default_mute_duration_secs", "max": 2419200, "min": 60, "type": "number", "unit": "s", "label": "Duree mute par defaut", "default": "3600", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Duree du mute si le mod ne specifie pas de duree dans /mute. Max Discord : 28 jours."}, {"key": "default_ban_duration_secs", "max": 31536000, "min": 0, "type": "number", "unit": "s", "label": "Duree ban par defaut", "default": "0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Duree d un ban temporaire par defaut. 0 = ban permanent."}, {"key": "ban_cleanup_interval", "max": 60, "min": 1, "type": "number", "unit": "min", "label": "Worker : intervalle scan bans expires", "default": "1", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence du scan des bans expires pour les lever automatiquement. Recommande : 1."}, {"key": "ban_delete_message_days", "max": 7, "min": 0, "type": "number", "unit": "j", "label": "Nb jours messages supprimes au ban", "default": "0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Lors d un ban, supprime les messages des N derniers jours. 0 = aucun, 7 = max Discord."}, {"key": "max_mute_duration_secs", "max": 2419200, "min": 60, "type": "number", "unit": "s", "label": "Duree max d un mute", "default": "2419200", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Plafond de duree d un mute. Max Discord = 28 jours (2419200s)."}, {"key": "reason_templates", "type": "text", "label": "Templates de raisons", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Raisons de sanction predefinies (autocomplete). Format : label|raison, une par ligne. Gere aussi via /template."}, {"key": "copilot_enabled", "type": "boolean", "label": "Copilote de modération", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active la commande /copilote (fiche membre + suggestion de sanction proportionnée basée sur l''historique et la jurisprudence du serveur)."}, {"key": "copilot_lookback_days", "max": 365, "min": 1, "type": "number", "unit": "jours", "label": "Copilote — fenêtre d''historique", "default": "90", "required": false, "depends_on": {"key": "copilot_enabled", "equals": "true"}, "description": "Ancienneté max des précédents pris en compte."}, {"key": "copilot_min_precedents", "max": 100, "min": 1, "type": "number", "label": "Copilote — précédents minimum", "default": "3", "required": false, "depends_on": {"key": "copilot_enabled", "equals": "true"}, "description": "Nombre de cas similaires requis avant de suggérer sur la jurisprudence (sinon repli sur l''échelle d''escalade)."}, {"key": "log_channel_id", "type": "channel", "label": "Salon des logs détaillés (carte complète)", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon de la carte détaillée d une sanction (cible, modérateur, gravité, strikes, raison). Indépendant du salon récap."}, {"key": "sanctions_log_channel_id", "type": "channel", "label": "Salon du récap des sanctions (carte 2 lignes)", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon de la carte récap 2 lignes (qui a sanctionné qui + raison). À mettre dans un salon différent, par ex. un salon commandes. Vide = désactivé."}, {"key": "appeal_category_id", "type": "category", "label": "Catégorie des salons d appel", "default": "", "required": false, "description": "Catégorie sous laquelle un salon privé est créé automatiquement quand un membre conteste sa sanction. Vide = simple notification dans le salon d appels."}, {"key": "sursis_role_id", "type": "role", "label": "Rôle Sursis (ban avec appel)", "default": "", "required": false, "description": "Rôle donné au membre lors d un /ban-sursis. Configure ce rôle pour qu il ne voie que le règlement. Requis pour le ban avec appel."}, {"key": "sursis_appeal_days", "type": "number", "label": "Délai d appel avant ban définitif (jours)", "default": "7", "required": false, "description": "Nombre de jours laissés au membre pour contester avant le bannissement automatique."}, {"key": "appeal_cancel_quorum", "type": "number", "label": "Votes modo requis pour annuler une sanction", "default": "2", "required": false, "description": "Nombre de moderateurs distincts qui doivent voter avant qu un administrateur puisse valider l annulation d une sanction."}, {"key": "appeal_guidelines", "type": "text", "label": "Texte du mode d emploi de l appel", "default": "", "required": false, "description": "Regles affichees dans le salon d appel (preuves attendues, droits/devoirs, conflit d interet). Markdown supporte. Vide = texte par defaut."}, {"key": "mod_quota_max", "max": 1000, "min": 0, "type": "number", "unit": "actions", "label": "Quota d actions par moderateur", "default": "0", "required": false, "description": "Nombre max d actions (ban/kick/mute/warn) qu un moderateur peut poser sur la fenetre. 0 = illimite (desactive)."}, {"key": "mod_quota_window_secs", "max": 86400, "min": 60, "type": "number", "unit": "s", "label": "Fenetre du quota", "default": "3600", "required": false, "depends_on": {"key": "mod_quota_max", "not_equals": "0"}, "description": "Duree de la fenetre glissante du quota, en secondes (defaut 3600 = 1h)."}]');
INSERT INTO public.bot_definitions VALUES ('monitoring', 'Surveillance', 'Detecte les bots et workers offline via leurs heartbeats Redis et publie les events de transition (online/offline) consommes par la page Securite et les alertes Discord.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Si OFF : pas de detection offline, pas d alertes en cas de bot crash."}, {"key": "check_interval", "max": 600, "min": 5, "type": "number", "unit": "s", "label": "Intervalle de verification", "default": "30", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence du check des heartbeats. 30s = bon compromis reactivite/charge."}]');
INSERT INTO public.bot_definitions VALUES ('voice-bot', 'Vocaux', 'Salons vocaux temporaires', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Active le systeme de salons vocaux temporaires (lobby create -> join -> salon perso)."}, {"key": "public_creator_channel_id", "type": "voice", "label": "Lobby salon public", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon vocal lobby : le rejoindre cree un nouveau salon temporaire public."}, {"key": "private_creator_channel_id", "type": "voice", "label": "Lobby salon prive", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon vocal lobby : le rejoindre cree un salon prive (acces sur invitation)."}, {"key": "game_creator_channel_id", "type": "voice", "label": "Lobby salon de jeu", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon vocal lobby : le rejoindre cree un salon de jeu (categorie dediee). Laisser vide pour desactiver."}, {"key": "afk_enabled", "type": "boolean", "label": "AFK sweep actif", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Tache periodique qui deplace/kick les membres AFK (self_mute + self_deaf trop longtemps)."}, {"key": "afk_timeout_minutes", "max": 1440, "min": 1, "type": "number", "unit": "min", "label": "Delai AFK", "default": "10", "required": false, "depends_on": {"key": "afk_enabled", "equals": "true"}, "description": "Apres combien de minutes en self_mute + self_deaf un membre est considere AFK."}, {"key": "afk_channel_id", "type": "voice", "label": "Salon AFK", "required": false, "depends_on": {"key": "afk_enabled", "equals": "true"}, "description": "Salon vocal ou les membres AFK sont deplaces."}, {"key": "afk_move_owner", "type": "boolean", "label": "Deplacer aussi les owners", "default": "false", "required": false, "depends_on": {"key": "afk_enabled", "equals": "true"}, "description": "Si OFF, le proprietaire d un salon temporaire ne sera jamais deplace en AFK (evite que le salon se ferme)."}, {"key": "voice_creation_cooldown_secs", "max": 600, "min": 0, "type": "number", "unit": "s", "label": "Cooldown creation salon", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Delai minimum entre 2 creations de salon par un meme user (anti-spam)."}, {"key": "voice_empty_cleanup_delay_secs", "max": 60, "min": 0, "type": "number", "unit": "s", "label": "Delai suppression salon vide", "default": "2", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Anti-race : on attend N secondes avant de supprimer un salon vide (le owner peut revenir vite)."}, {"key": "voice_flood_max_messages", "max": 50, "min": 1, "type": "number", "label": "Seuil flood (messages)", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre de clics panel admin dans la fenetre avant mute auto."}, {"key": "voice_flood_time_window_secs", "max": 60, "min": 1, "type": "number", "unit": "s", "label": "Fenetre flood", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "voice_flood_mute_duration_secs", "max": 3600, "min": 30, "type": "number", "unit": "s", "label": "Duree mute si flood", "default": "30", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "voice_vote_kick_timeout_secs", "max": 600, "min": 30, "type": "number", "unit": "s", "label": "Duree vote-kick", "default": "60", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Apres ce delai, le vote-kick expire automatiquement (sans verdict)."}, {"key": "voice_anchor_category_id", "type": "category", "label": "Categorie des salons temporaires", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Les salons vocaux temporaires seront crees dans cette categorie (en bas). Vide = racine du serveur."}, {"key": "panel_post_enabled", "type": "boolean", "label": "Poster le panneau de controle dans le chat vocal", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si OFF, aucun panneau de controle n est poste a la creation d un salon vocal temporaire."}, {"key": "log_channel_id", "type": "channel", "label": "Salon des logs vocaux", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon textuel ou sont postees les cartes de session (creation, arrivees/departs, fermeture des salons vocaux temporaires). Vide = pas de logs."}, {"key": "observed_voice_channels", "type": "voice_list", "label": "Vocaux permanents a observer pour les logs (IDs separes par virgule)", "default": "", "required": false}, {"key": "afk_sweep_interval_secs", "max": 600, "min": 30, "type": "number", "unit": "s", "label": "AFK — intervalle du balayage", "default": "60", "required": false, "description": "Frequence a laquelle le bot verifie les membres AFK a deplacer (lecture globale : premiere guild configuree)."}, {"key": "voice_ban_preset_secs", "type": "text", "label": "Voice-ban — presets de duree (CSV secondes)", "default": "300,3600,86400", "required": false, "description": "Trois durees (en secondes) des boutons de voice-ban, separees par des virgules. Defaut : 300,3600,86400 (5 min, 1 h, 24 h)."}, {"key": "voice_max_user_limit", "max": 99, "min": 1, "type": "number", "label": "Salon vocal — limite max de membres", "default": "99", "required": false, "description": "Limite maximale de membres autorisee pour un salon vocal (plafond Discord : 99)."}]');
INSERT INTO public.bot_definitions VALUES ('progression-bot', 'Progression', 'Suivi des messages, temps vocal, XP, niveaux et progression', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Active le systeme XP / niveaux. Si OFF : aucun XP n est attribue."}, {"key": "xp_per_message", "max": 1000, "min": 0, "type": "number", "unit": "XP", "label": "XP par message", "default": "15", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "xp_cooldown_secs", "max": 3600, "min": 0, "type": "number", "unit": "s", "label": "Cooldown XP message", "default": "60", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Delai min entre 2 gains XP par message (anti-farm)."}, {"key": "xp_per_voice_minute", "max": 100, "min": 0, "type": "number", "unit": "XP/min", "label": "XP par minute en vocal", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "min_message_length", "max": 200, "min": 0, "type": "number", "unit": "chars", "label": "Longueur min message", "default": "3", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Message plus court = pas d XP (anti-spam 1 lettre)."}, {"key": "xp_channel_multipliers", "type": "text", "label": "Multiplicateurs XP par salon (CSV)", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Format : channel_id:mult,channel_id:mult (ex: 12345:2,67890:0.5)."}, {"key": "xp_role_multipliers", "type": "text", "label": "Multiplicateurs XP par role (CSV)", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Format : role_id:mult,role_id:mult."}, {"key": "default_role_ids", "type": "text", "label": "Roles attribues par defaut (CSV)", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Roles donnes a chaque nouveau membre. IDs separes par virgules."}, {"key": "ignored_channels", "type": "channel_list", "label": "Salons ignores (CSV)", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Aucun XP ne sera attribue dans ces salons. IDs separes par virgules."}, {"key": "ignored_roles", "type": "role_list", "label": "Roles ignores (CSV)", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Membres avec ces roles ne gagneront pas d XP. IDs separes par virgules."}, {"key": "streak_enabled", "type": "boolean", "label": "Streaks de connexion", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Tracke les jours consecutifs d activite + applique un multiplicateur XP croissant."}, {"key": "level_up_channel_id", "type": "channel", "label": "Salon annonce level-up", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si vide, l annonce est postee dans le salon courant (ou pas si annonce desactivee)."}, {"key": "levelup_announce_enabled", "type": "boolean", "label": "Annonce level-up dans le salon", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si OFF, aucun message de level-up n est poste dans le salon."}, {"key": "levelup_dm_enabled", "type": "boolean", "label": "DM lors du level-up", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si ON, envoie aussi un DM au membre lors du level-up (en plus de l annonce dans le salon)."}, {"key": "levelup_message", "type": "text", "label": "Message custom level-up", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Template du message level-up. Variables : {user}, {level}, {kind}. Si vide, message par defaut. S applique a la fois a l annonce salon et au DM."}, {"key": "max_level", "max": 1000, "min": 0, "type": "number", "label": "Niveau max (0 = illimite)", "default": "0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Niveau max au-dela duquel les annonces level-up sont supprimees. 0 = illimite. Les role rewards continuent par contre."}, {"key": "monthly_ranking_enabled", "type": "boolean", "label": "Publier le classement mensuel (texte/vocal/global) sur Discord", "default": "false", "required": false}, {"key": "monthly_ranking_channel_id", "type": "channel", "label": "Salon de publication du classement mensuel", "required": false}, {"key": "monthly_ranking_top_count", "type": "number", "label": "Nombre de membres affiches dans le classement mensuel", "default": "10", "required": false}, {"key": "staff_prefix_enabled", "type": "boolean", "label": "Emoji de role staff devant le pseudo", "default": "false", "required": false, "description": "Ajoute automatiquement un emoji devant le pseudo selon le role staff le plus eleve du membre (ex. 👑 admin). Se combine avec le prefixe de niveau [NN]. Emojis unicode uniquement."}, {"key": "staff_role_emojis", "type": "text", "label": "Emojis par role (CSV)", "required": false, "depends_on": {"key": "staff_prefix_enabled", "equals": "true"}, "description": "Format role_id:emoji, separes par des virgules. Ex: 111:👑,222:🛡️,333:⚔️. L''emoji du role le plus haut du membre est utilise. Emojis unicode uniquement (les emojis custom ne s''affichent pas dans un pseudo)."}, {"key": "streak_bonus_per_week", "max": 10, "min": 0, "type": "number", "label": "Streak — bonus XP par semaine", "default": "0.1", "required": false, "description": "Bonus de multiplicateur XP ajoute par semaine complete de streak (0.1 = +10% par 7 jours consecutifs)."}, {"key": "streak_max_multiplier", "max": 10, "min": 1, "type": "number", "label": "Streak — multiplicateur max", "default": "1.5", "required": false, "description": "Plafond du multiplicateur XP de streak (1.5 = +50% max). Garde >= 1.0 (ne reduit jamais l XP)."}, {"key": "monthly_ranking_excluded_roles", "type": "role_list", "label": "Roles exclus du classement mensuel", "required": false}]');
INSERT INTO public.bot_definitions VALUES ('ticket-bot', 'Tickets', 'Systeme d assistance par tickets', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Active le systeme de tickets de support / appels de sanction."}, {"key": "assistance_channel_id", "type": "channel", "label": "Salon assistance", "required": true, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon ou le panneau d ouverture de ticket est poste."}, {"key": "ticket_category_id", "type": "category", "label": "Categorie tickets", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Categorie Discord ou les salons tickets sont crees. Vide = pas de categorie."}, {"key": "admin_role_id", "type": "role", "label": "Role Administrateur", "required": true, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "moderator_role_id", "type": "role", "label": "Role Moderateur", "required": true, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "max_open_per_user", "max": 50, "min": 0, "type": "number", "label": "Max tickets ouverts par user", "default": "0", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "0 = illimite. Au-dela, le user ne peut plus en ouvrir."}, {"key": "inactive_close_days", "max": 90, "min": 0, "type": "number", "unit": "j", "label": "Fermeture auto si inactif", "default": "7", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "0 = desactive. Tickets sans activite > N jours sont fermes par le worker."}, {"key": "close_delay_secs", "max": 600, "min": 0, "type": "number", "unit": "s", "label": "Delai avant suppression salon", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Apres validation de fermeture, on attend N secondes avant de delete le salon."}, {"key": "welcome_message", "type": "text", "label": "Message d accueil custom", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Vide = message par defaut. Affiche dans le salon ticket a sa creation."}, {"key": "transcript_dm_enabled", "type": "boolean", "label": "Transcript en DM a la fermeture", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Envoie le transcript du ticket en DM a son auteur a la fermeture."}, {"key": "transcript_format", "type": "enum", "label": "Format transcript", "default": "text", "options": [{"label": "Texte simple", "value": "text"}, {"label": "Markdown", "value": "markdown"}, {"label": "HTML (TODO)", "value": "html"}], "required": false, "depends_on": {"key": "transcript_dm_enabled", "equals": "true"}, "description": "text = plain (mail/sms), markdown = format Discord avec **bold** et > quote (default), html = document HTML envoye en attachment .html."}, {"key": "satisfaction_enabled", "type": "boolean", "label": "Sondage satisfaction (1-5 etoiles)", "default": "true", "required": false, "depends_on": {"key": "transcript_dm_enabled", "equals": "true"}, "description": "A la fermeture, envoie un sondage avec 5 boutons etoiles dans le DM transcript. Necessite transcript_dm_enabled."}, {"key": "sla_escalation_minutes", "max": 1440, "min": 0, "type": "number", "unit": "min", "label": "Delai escalade SLA appels", "default": "60", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "0 = desactive. Tickets de type \"appel_sanction\" sans premiere reponse > N minutes sont escalades par le worker."}, {"key": "sla_first_response_minutes", "max": 1440, "min": 0, "type": "number", "unit": "min", "label": "SLA premiere reponse", "default": "30", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Apres N min sans premiere reponse, le bot poste un rappel dans le ticket (avant escalation). 0 = desactive."}, {"key": "appeal_sla_scan_interval", "max": 3600, "min": 30, "type": "number", "unit": "s", "label": "Worker : intervalle scan SLA appels", "default": "300", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence de scan des tickets d appel pour detection de depassement SLA."}, {"key": "response_templates", "type": "text", "label": "Templates de reponses", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Format CSV multilignes : label|contenu (un par ligne). Disponibles via /ticket reponse."}, {"key": "faq_entries", "type": "text", "label": "FAQ", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Format CSV multilignes : question|reponse (une par ligne). Affiche dans /faq."}, {"key": "color_normal", "type": "text", "label": "Couleur ticket normal (hex)", "default": "2ecc71", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "color_urgent", "type": "text", "label": "Couleur ticket urgent (hex)", "default": "ff6600", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "color_confidential", "type": "text", "label": "Couleur ticket confidentiel (hex)", "default": "e74c3c", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "color_staff", "type": "text", "label": "Couleur embed staff (hex)", "default": "e67e22", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "color_user", "type": "text", "label": "Couleur embed user (hex)", "default": "3498db", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "ticket_subject_min_len", "max": 4000, "min": 1, "type": "number", "unit": "caracteres", "label": "Sujet — longueur min", "default": "5", "required": false, "description": "Longueur minimale du champ Sujet de la modale de ticket. Doit rester <= au max (sinon les defauts 5/100 sont utilises)."}, {"key": "ticket_subject_max_len", "max": 4000, "min": 1, "type": "number", "unit": "caracteres", "label": "Sujet — longueur max", "default": "100", "required": false, "description": "Longueur maximale du champ Sujet de la modale de ticket (plafonnee a 4000, limite Discord)."}, {"key": "ticket_desc_min_len", "max": 4000, "min": 1, "type": "number", "unit": "caracteres", "label": "Description — longueur min", "default": "10", "required": false, "description": "Longueur minimale du champ Description de la modale de ticket. Doit rester <= au max (sinon les defauts 10/2000 sont utilises)."}, {"key": "ticket_desc_max_len", "max": 4000, "min": 1, "type": "number", "unit": "caracteres", "label": "Description — longueur max", "default": "2000", "required": false, "description": "Longueur maximale du champ Description de la modale de ticket (plafonnee a 4000, limite Discord)."}, {"key": "ticket_owner_ids", "type": "text", "label": "Proprietaires (IDs, pour Probleme moderateur)", "default": "", "required": false, "description": "IDs Discord des proprietaires/co-fondateurs (separes par des virgules) qui recoivent les tickets Probleme avec un moderateur. L owner du serveur est toujours inclus automatiquement."}]');
INSERT INTO public.bot_definitions VALUES ('cache', 'Cache Redis', 'Pre-calcul des donnees analytics, dashboard, leaderboards et stats vocales dans Redis pour des reponses instantanees cote frontend. Sans ce module, les requetes lourdes vont directement en DB a chaque vue.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Si OFF, le frontend tape la DB en direct (lent sur grosses guilds)."}, {"key": "analytics_cache_refresh", "max": 3600, "min": 30, "type": "number", "unit": "s", "label": "Refresh cache analytics", "default": "300", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence de regeneration du cache analytics. 300s = 5 min."}, {"key": "dashboard_cache_refresh", "max": 3600, "min": 30, "type": "number", "unit": "s", "label": "Refresh cache dashboard", "default": "600", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "voice_stats_cache_refresh", "max": 86400, "min": 60, "type": "number", "unit": "s", "label": "Refresh stats vocales", "default": "3600", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "leaderboards_refresh", "max": 3600, "min": 30, "type": "number", "unit": "s", "label": "Refresh leaderboards", "default": "300", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence de regeneration du cache leaderboards (top XP, top voice, etc.)."}, {"key": "user_cache_sync", "max": 3600, "min": 60, "type": "number", "unit": "s", "label": "Sync cache utilisateurs", "default": "600", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence de synchronisation du cache des usernames Discord."}, {"key": "partition_manager", "max": 86400, "min": 600, "type": "number", "unit": "s", "label": "Gestion partitions Postgres", "default": "3600", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence de maintenance des partitions Postgres (creation/drop des tranches mensuelles)."}]');
INSERT INTO public.bot_definitions VALUES ('community-bot', 'Communaute (parrainage + roles)', 'Systeme de parrainage entre membres + verification de roles prerequis (ex: avoir le role Verifie pour poster). Les groupes exclusifs empechent la coexistence de roles incompatibles.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false}, {"key": "max_sponsorships", "max": 100, "min": 1, "type": "number", "label": "Max parrainages par membre", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Combien de filleuls un meme parrain peut avoir simultanement."}, {"key": "sponsor_min_parrain_days", "max": 365, "min": 0, "type": "number", "unit": "j", "label": "Anciennete min parrain", "default": "30", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Le parrain doit avoir au moins N jours d anciennete sur le serveur pour parrainer."}, {"key": "sponsor_max_filleul_days", "max": 365, "min": 0, "type": "number", "unit": "j", "label": "Anciennete max filleul", "default": "7", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Le filleul doit avoir moins de N jours sur le serveur (recent join)."}, {"key": "exclusive_groups", "type": "text", "label": "Groupes exclusifs", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Groupes de roles incompatibles. Format CSV multilignes : group_name|role_id,role_id (un par ligne). Si un membre a deja un role d un groupe et reussit en obtient un autre du meme groupe, l ancien est retire."}, {"key": "role_prerequisites", "type": "text", "label": "Prerequis de roles", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Format CSV : target_role_id:requires_role_id (un par ligne). Pour obtenir target, il faut deja avoir requires."}, {"key": "temp_roles", "type": "text", "label": "Roles temporaires (assign manuel)", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Format CSV : role_id:duree_secs (un par ligne). Roles attribues via /role temp qui seront retires apres N secondes par le worker temp_roles."}, {"key": "sponsor_cooldown_secs", "max": 3600, "min": 0, "type": "number", "unit": "s", "label": "Parrainage : cooldown /parrain", "default": "30", "required": false, "description": "Delai minimum entre deux commandes /parrain pour un meme membre (anti-spam)."}, {"key": "role_button_cooldown_secs", "max": 3600, "min": 0, "type": "number", "unit": "s", "label": "Panneaux de roles : cooldown bouton", "default": "2", "required": false, "description": "Delai minimum entre deux clics sur un bouton de role (anti-spam du toggle)."}]');
INSERT INTO public.bot_definitions VALUES ('ai-dataset-bot', 'Collecte Dataset IA', 'Collecte tous les messages texte des salons (sauf bots) pour entrainer un modele IA. Desactive par defaut. Activez ponctuellement pour preparer un dataset, puis desactivez et exportez via la page Dataset IA.', '[]');
INSERT INTO public.bot_definitions VALUES ('rotation-bot', 'Administrateur tournant', 'Chaque periode, un moderateur devient administrateur a tour de role (acceptation du modo + validation de l owner).', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "false", "required": false, "description": "Active la rotation automatique de l administrateur."}, {"key": "mod_role_id", "type": "role", "label": "Role Moderateur (pool)", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Les membres ayant ce role sont les candidats a la rotation."}, {"key": "admin_role_id", "type": "role", "label": "Role Administrateur (attribue)", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Role donne au modo selectionne (et retire au precedent, qui redevient Moderateur)."}, {"key": "period_days", "max": 366, "min": 1, "type": "number", "unit": "jours", "label": "Duree d un mandat", "default": "30", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Au bout de cette duree, on lance une nouvelle rotation."}, {"key": "response_timeout_hours", "max": 720, "min": 1, "type": "number", "unit": "heures", "label": "Delai de reponse", "default": "72", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Temps laisse au modo (et a l owner) pour repondre avant de passer au suivant."}, {"key": "objective_message", "type": "text", "label": "Message / objectif (MP au modo)", "default": "Ce mois-ci, c est ton tour de devenir Administrateur ! Ton objectif : animer le serveur, veiller au respect des regles et accompagner la communaute. Acceptes-tu ce mandat ?", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Texte envoye en MP au candidat. Tu peux expliquer son objectif/mandat."}]');
INSERT INTO public.bot_definitions VALUES ('command-channel-bot', 'Salons a commandes', 'Salons ou seules les commandes sont autorisees : tout message classique est supprime (sauf owner et bots).', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "false", "required": false, "description": "Active la suppression des messages classiques dans les salons designes."}, {"key": "command_channels", "type": "channel_list", "label": "Salons a commandes uniquement", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salons ou seules les commandes sont autorisees. Tout message classique y est supprime en silence (sauf owner et bots)."}]');
INSERT INTO public.bot_definitions VALUES ('welcome-bot', 'Welcome', 'Accueil des nouveaux membres — bienvenue, depart, reglement, compteur, anniversaires.', '[{"key": "welcome_enabled", "type": "boolean", "label": "Message de bienvenue", "default": "true", "required": false, "description": "Envoyer un embed de bienvenue dans le salon configure quand un nouveau membre rejoint."}, {"key": "welcome_channel_id", "type": "channel", "label": "Salon de bienvenue", "required": false, "description": "Salon ou est poste le message de bienvenue."}, {"key": "welcome_message", "type": "text", "label": "Message de bienvenue", "default": "Bienvenue {user} sur **{server}** ! Tu es le **{count}e** membre.", "required": false, "description": "Variables : {user}, {server}, {count}."}, {"key": "welcome_embed_color", "type": "text", "label": "Couleur embed (hex)", "default": "3498db", "required": false, "description": "Code couleur hex sans # (ex: 3498db)."}, {"key": "welcome_dm_enabled", "type": "boolean", "label": "DM de bienvenue", "default": "false", "required": false, "description": "Envoyer aussi un message prive au nouveau membre."}, {"key": "welcome_dm_message", "type": "text", "label": "Message DM de bienvenue", "default": "Bienvenue sur **{server}** ! N oublie pas de lire les regles.", "required": false, "description": "Variables : {user}, {server}, {count}."}, {"key": "rejoin_message", "type": "text", "label": "Message retour (rejoin)", "default": "Content de te revoir {user} ! Tu nous avais manque.", "required": false, "description": "Message affiche quand un membre deja connu re-rejoint le serveur."}, {"key": "leave_enabled", "type": "boolean", "label": "Message de depart", "default": "true", "required": false, "description": "Envoyer un embed quand un membre quitte le serveur."}, {"key": "leave_channel_id", "type": "channel", "label": "Salon de depart", "required": false}, {"key": "leave_message", "type": "text", "label": "Message de depart", "default": "{user} nous a quittes. Nous sommes maintenant **{count}** membres.", "required": false, "description": "Variables : {user}, {server}, {count}."}, {"key": "rules_enabled", "type": "boolean", "label": "Validation du reglement", "default": "false", "required": false, "description": "Afficher un bouton d acceptation du reglement qui attribue un role."}, {"key": "rules_channel_id", "type": "channel", "label": "Salon du reglement", "required": false}, {"key": "rules_message", "type": "text", "label": "Message du reglement", "default": "Lis les regles et clique sur le bouton pour acceder au serveur.", "required": false}, {"key": "rules_button_label", "type": "text", "label": "Libelle du bouton reglement", "default": "J accepte les regles", "required": false}, {"key": "counter_enabled", "type": "boolean", "label": "Compteur de membres", "default": "false", "required": false, "description": "Renomme un canal vocal avec le nombre de membres."}, {"key": "counter_channel_id", "type": "voice", "label": "Canal vocal compteur", "required": false}, {"key": "counter_format", "type": "text", "label": "Format compteur", "default": "Membres : {count}", "required": false, "description": "Variable : {count}."}, {"key": "voice_counter_enabled", "type": "boolean", "label": "Compteur de membres en vocal", "default": "false", "required": false, "description": "Renomme un salon avec le nombre de membres actuellement connectes en vocal."}, {"key": "voice_counter_channel_id", "type": "voice", "label": "Salon compteur vocal", "required": false}, {"key": "voice_counter_format", "type": "text", "label": "Format compteur vocal", "default": "En Vocal : {count}", "required": false, "description": "Variable : {count}."}, {"key": "anniversary_enabled", "type": "boolean", "label": "Anniversaires serveur", "default": "false", "required": false, "description": "Souhaiter un anniversaire d arrivee aux membres chaque annee."}, {"key": "anniversary_channel_id", "type": "channel", "label": "Salon anniversaires", "required": false}, {"key": "anniversary_message", "type": "text", "label": "Message anniversaire", "default": "Felicitations {user}, ca fait **{years} an(s)** que tu es sur **{server}** !", "required": false, "description": "Variables : {user}, {server}, {years}."}, {"key": "rules_role_id", "type": "role_list", "label": "Roles apres validation", "required": false, "description": "Roles attribues quand un membre clique sur le bouton d acceptation. Tu peux en choisir plusieurs."}, {"key": "age_min", "max": 120, "min": 0, "type": "number", "label": "Verification age — age minimum saisissable", "default": "5", "required": false, "depends_on": {"key": "age_check_enabled", "equals": "true"}, "description": "Borne basse acceptee dans le formulaire de verification d age (valeurs plus petites = rejet de la saisie)."}, {"key": "age_max", "max": 200, "min": 0, "type": "number", "label": "Verification age — age maximum saisissable", "default": "120", "required": false, "depends_on": {"key": "age_check_enabled", "equals": "true"}, "description": "Borne haute acceptee dans le formulaire de verification d age (valeurs plus grandes = rejet de la saisie)."}, {"key": "age_ban_days_per_year", "max": 366, "min": 1, "type": "number", "label": "Verification age — jours de ban par annee manquante", "default": "365", "required": false, "depends_on": {"key": "age_check_enabled", "equals": "true"}, "description": "Duree (en jours) du ban temporaire par annee manquante sous l age minimum. 365 = un an par annee."}, {"key": "leave_embed_color", "type": "text", "label": "Couleur embed depart (hex)", "default": "e74c3c", "required": false, "description": "Code couleur hex sans # de l embed de message de depart (ex: e74c3c)."}, {"key": "rules_embed_color", "type": "text", "label": "Couleur embed reglement (hex)", "default": "5865f2", "required": false, "description": "Code couleur hex sans # du panneau de reglement (ex: 5865f2)."}, {"key": "age_ban_log_channel_id", "type": "channel", "label": "Salon de log des bans d age", "required": false, "depends_on": {"key": "age_check_enabled", "equals": "true"}, "description": "Salon ou le bot poste une card quand un membre est banni par la verification d age (age saisi sous le minimum). Affiche l age declare, le minimum, la duree du ban et la date de deban auto. Vide = pas de log."}]');
INSERT INTO public.bot_definitions VALUES ('audit-bot', 'Audit Bot', 'Bot d''audit — logs avances des evenements serveur', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Active les logs d audit avances + cache Redis pour les watched users."}, {"key": "log_channel_id", "type": "channel", "label": "Salon de logs (general / fallback)", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon de logs par defaut. Utilise comme fallback quand un salon plus specifique (anomaly/join_leave/profile_edit) n est pas configure."}, {"key": "anomaly_channel_id", "type": "channel", "label": "Salon anomalies", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon des alertes urgentes (mass_ban, mass_delete, mass_role_change). Si vide -> log_channel_id."}, {"key": "join_leave_channel_id", "type": "channel", "label": "Salon joins / leaves", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon ou poster les arrivees / departs de membres. Si vide -> log_channel_id."}, {"key": "profile_edit_channel_id", "type": "channel", "label": "Salon modifications profil", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nickname, avatar, banner, pseudo Discord global. Si vide -> log_channel_id."}, {"key": "message_cache_size", "max": 100000, "min": 100, "type": "number", "label": "Taille cache messages", "default": "10000", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Buffer in-memory des messages recents pour les detections d anomalie. Per-guild override applique au prochain redemarrage du bot."}, {"key": "audit_cache_refresh_interval", "max": 3600, "min": 10, "type": "number", "unit": "s", "label": "Refresh cache watched users", "default": "60", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence de sync Redis -> bot du cache des utilisateurs surveilles."}, {"key": "audit_sync_interval", "max": 3600, "min": 60, "type": "number", "unit": "s", "label": "Worker : intervalle sync audit Discord", "default": "300", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence de polling des audit logs Discord via l API REST (rattrape les events rates en gateway)."}, {"key": "anomaly_enabled", "type": "boolean", "label": "Detection d''anomalies", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Detecte les comportements en rafale (mass ban/delete/role change)."}, {"key": "anomaly_mass_ban_threshold", "max": 100, "min": 1, "type": "number", "label": "Seuil mass ban (par 60s)", "default": "5", "required": false, "depends_on": {"key": "anomaly_enabled", "equals": "true"}}, {"key": "anomaly_mass_delete_threshold", "max": 1000, "min": 1, "type": "number", "label": "Seuil mass delete (par 60s)", "default": "20", "required": false, "depends_on": {"key": "anomaly_enabled", "equals": "true"}}, {"key": "anomaly_mass_role_threshold", "max": 100, "min": 1, "type": "number", "label": "Seuil mass role change (par 60s)", "default": "10", "required": false, "depends_on": {"key": "anomaly_enabled", "equals": "true"}}, {"key": "weekly_report_enabled", "type": "boolean", "label": "Rapport hebdomadaire (/audit stats)", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active la commande /audit stats qui affiche un recap hebdo."}, {"key": "message_log_channel_id", "type": "channel", "label": "Salon de logs des messages (édition / suppression)", "default": "", "required": false, "description": "Salon où le bot poste un embed quand un message est édité (avant/après) ou supprimé. Si vide, utilise log_channel_id."}, {"key": "voice_log_channel_id", "type": "channel", "label": "Salon de logs vocaux (connexion / deconnexion)", "default": "", "required": false, "description": "Salon ou le bot poste un embed colore a chaque connexion (vert), deconnexion (rouge) ou deplacement (bleu) vocal, pour TOUS les salons vocaux. Si vide, utilise log_channel_id."}, {"key": "command_log_enabled", "type": "boolean", "label": "Log des commandes admin/moderateur", "default": "false", "required": false, "description": "Poste une ligne quand une commande admin/moderateur est utilisee."}, {"key": "command_log_channel_id", "type": "channel", "label": "Salon du log des commandes admin", "default": "", "required": false, "description": "Salon ou poster le log des commandes admin/moderateur. Requis pour activer le log."}, {"key": "role_log_window_secs", "type": "number", "label": "Fenetre carte roles (secondes)", "default": "300", "required": false, "description": "Duree pendant laquelle la carte de changement de roles reste active et se met a jour (fenetre glissante). Evite le spam d une carte par role. Defaut 300 (5 min)."}]');
INSERT INTO public.bot_definitions VALUES ('ai', 'IA (texte + vision)', 'Traite les jobs IA en arriere-plan : analyse texte (toxicite, spam, hate speech) et vision (NSFW, violence). Les bots publient des jobs dans la file ai_jobs, le worker les depile et appelle les modeles ONNX.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Si OFF : aucun job IA n est traite. Les bots qui publient des jobs verront leur file s accumuler."}, {"key": "ai_poll_interval", "max": 60, "min": 1, "type": "number", "unit": "s", "label": "Intervalle polling jobs IA", "default": "5", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence de depilage de la file ai_jobs."}, {"key": "ai_job_timeout", "max": 600, "min": 5, "type": "number", "unit": "s", "label": "Timeout job IA", "default": "60", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Duree max d un job IA avant de le marquer failed."}, {"key": "ai_batch_size", "max": 100, "min": 1, "type": "number", "unit": "jobs", "label": "Worker : taille du batch de jobs IA", "default": "5", "required": false, "description": "Nombre de jobs IA claimes et traites a chaque tick de depilage. Plus haut = meilleur debit mais plus de charge sur l API d inference."}]');
INSERT INTO public.bot_definitions VALUES ('confessions', 'Confessions anonymes', 'Permet aux membres de poster des confessions anonymes via un panel. La configuration (salon de publication, panel_message_id) est automatiquement persistee par /confess-admin deploy-panel.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Active les commandes /confess + /confess-admin. Le salon de publication est defini via /confess-admin deploy-panel."}, {"key": "thread_archive_minutes", "type": "enum", "label": "Thread de reponses — archivage auto", "default": "60", "options": [{"label": "1 heure", "value": "60"}, {"label": "1 jour", "value": "1440"}, {"label": "3 jours", "value": "4320"}, {"label": "1 semaine", "value": "10080"}], "required": false, "description": "Delai d inactivite apres lequel le thread de reponses d une confession s archive. Un thread archive se rouvre automatiquement a la prochaine reponse. Discord n autorise que ces 4 paliers."}, {"key": "report_reason_max_len", "max": 4000, "min": 1, "type": "number", "unit": "caracteres", "label": "Signalement — longueur max de la raison", "default": "500", "required": false, "description": "Longueur maximale du champ Raison de la modale de signalement d une confession (plafonnee a 4000, limite Discord)."}, {"key": "quota_window_hours", "max": 168, "min": 1, "type": "number", "unit": "heures", "label": "Quota — fenetre glissante (heures)", "default": "24", "required": false, "description": "Fenetre glissante (en heures) sur laquelle le nombre max de confessions par jour et par utilisateur est compte. Defaut 24h (bornee a >= 1h a l usage)."}]');
INSERT INTO public.bot_definitions VALUES ('bump-bot', 'Bump Rewards', 'Recompense des coins quand un membre fait /bump (Disboard), recompense graduee selon le nombre de bumps de la semaine, + rappel apres cooldown.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "false", "required": false, "description": "Active la recompense de bump."}, {"key": "bump_reward_base", "type": "number", "label": "Coins de base par bump", "default": "100", "required": false, "description": "Recompense du 1er bump de la semaine."}, {"key": "bump_reward_step", "type": "number", "label": "Bonus par bump suppl. dans la semaine", "default": "50", "required": false, "description": "Ajoute par bump au-dela du 1er (recompense graduee)."}, {"key": "bump_reward_max", "type": "number", "label": "Recompense maximale par bump", "default": "500", "required": false, "description": "Plafond de la recompense graduee."}, {"key": "bump_cooldown_minutes", "type": "number", "label": "Cooldown du bump (minutes)", "default": "120", "required": false, "description": "Delai Disboard entre deux bumps (defaut 120)."}, {"key": "bump_reminder_enabled", "type": "boolean", "label": "Rappel apres cooldown", "default": "true", "required": false, "description": "Poste un rappel dans le salon quand un nouveau bump est possible."}, {"key": "bump_channel_id", "type": "channel", "label": "Salon des bumps (annonce + rappel)", "default": "", "required": false, "description": "Salon ou poster la confirmation de recompense et le rappel. Si vide, utilise le salon du bump."}, {"key": "vip_enabled", "type": "boolean", "label": "Role VIP apres X bumps", "default": "false", "required": false, "description": "Attribue un role VIP a partir d un certain nombre de bumps cumules."}, {"key": "vip_role_id", "type": "role", "label": "Role VIP a attribuer", "default": "", "required": false, "description": "Role donne au membre une fois le seuil de bumps atteint."}, {"key": "vip_bump_threshold", "type": "number", "label": "Nombre de bumps pour devenir VIP", "default": "10", "required": false, "description": "Total de bumps (cumul) requis pour debloquer le role VIP."}, {"key": "disboard_enabled", "type": "boolean", "label": "Provider Disboard actif", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Recompense les bumps Disboard (necessite aussi le module actif)."}, {"key": "discordl_enabled", "type": "boolean", "label": "Provider DiscordL actif", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Recompense les bumps DiscordL (discordl.org) (necessite aussi le module actif)."}, {"key": "discordl_cooldown_minutes", "type": "number", "label": "Cooldown DiscordL (minutes)", "default": "240", "required": false, "depends_on": {"key": "discordl_enabled", "equals": "true"}, "description": "Delai DiscordL entre deux bumps (defaut 240 = 4h)."}, {"key": "discordl_vote_enabled", "type": "boolean", "label": "Provider DiscordL Vote actif", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Recompense les votes DiscordL (discordl.org) — meme bot que le bump, action /vote."}, {"key": "discordl_vote_cooldown_minutes", "type": "number", "label": "Cooldown DiscordL Vote (minutes)", "default": "720", "required": false, "depends_on": {"key": "discordl_vote_enabled", "equals": "true"}, "description": "Delai DiscordL entre deux votes (defaut 720 = 12h)."}]');
INSERT INTO public.bot_definitions VALUES ('help-bot', 'Panneau d''aide', 'Publie automatiquement dans un salon un catalogue de toutes les commandes du serveur (triées par categorie, avec description). Genere et mis a jour par le bot, sans intervention manuelle.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Publie/maintient les panneaux d aide."}, {"key": "admin_category_id", "type": "category", "label": "Catégorie — salon Admin", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Catégorie sous laquelle ranger le salon des commandes Admin. Vide = catégorie \"Aide commandes\" créée par le bot."}, {"key": "moderation_category_id", "type": "category", "label": "Catégorie — salon Modération", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Catégorie sous laquelle ranger le salon des commandes Modération. Vide = catégorie par défaut."}, {"key": "membres_category_id", "type": "category", "label": "Catégorie — salon Membres", "default": "", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Catégorie sous laquelle ranger le salon des commandes Membres. Vide = catégorie par défaut."}]');
INSERT INTO public.bot_definitions VALUES ('analytics', 'Analytics & snapshots', 'Genere les snapshots horaires et quotidiens (messages, voice, joins, sanctions) qui alimentent les graphiques du dashboard et le rapport mensuel.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Si OFF : pas de snapshots, les graphiques du dashboard restent figes."}, {"key": "track_voice_stats", "type": "boolean", "label": "Tracker stats vocales", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Inclut voice_minutes dans les snapshots quotidiens. Si OFF, la colonne reste a 0."}, {"key": "track_message_stats", "type": "boolean", "label": "Tracker stats messages", "default": "true", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Inclut messages dans les snapshots quotidiens et horaires. Si OFF, les colonnes restent a 0."}, {"key": "hourly_snapshot_interval", "max": 1440, "min": 10, "type": "number", "unit": "min", "label": "Intervalle snapshot horaire (minutes)", "default": "60", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence des snapshots horaires. Valeur en MINUTES."}, {"key": "daily_snapshot_interval", "max": 168, "min": 1, "type": "number", "unit": "h", "label": "Intervalle snapshot journalier (heures)", "default": "1", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Frequence des snapshots journaliers. Valeur en HEURES."}, {"key": "data_retention_days", "max": 3650, "min": 0, "type": "number", "unit": "j", "label": "Retention des donnees", "default": "90", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Apres combien de jours les snapshots (daily_activity, hourly_activity) sont supprimes. 0 = illimite. Le job de cleanup tourne 1x/jour."}, {"key": "top_users_count", "max": 100, "min": 1, "type": "number", "label": "Top utilisateurs (taille par defaut)", "default": "10", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Taille par defaut du top dans /api/analytics/top-infractors et dans le post Discord automatique."}, {"key": "export_format", "type": "enum", "label": "Format d''export par defaut", "default": "json", "options": [{"label": "JSON", "value": "json"}, {"label": "CSV", "value": "csv"}], "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Format par defaut de /api/analytics/export quand le client ne specifie pas ?format=. JSON pour API, CSV pour tableur."}, {"key": "top_users_publish_enabled", "type": "boolean", "label": "Publier le Top users sur Discord", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Active la publication automatique du Top users dans un salon Discord."}, {"key": "top_users_publish_channel_id", "type": "channel", "label": "Salon de publication Top users", "required": false, "depends_on": {"key": "top_users_publish_enabled", "equals": "true"}, "description": "Salon ou poster l embed Top users."}, {"key": "top_users_publish_interval_days", "max": 90, "min": 1, "type": "number", "unit": "j", "label": "Frequence publication Top users (jours)", "default": "7", "required": false, "depends_on": {"key": "top_users_publish_enabled", "equals": "true"}, "description": "Intervalle minimal entre deux publications. Le worker tick chaque heure et publie quand l interval est ecoule."}, {"key": "monthly_ranking_check_secs", "max": 86400, "min": 300, "type": "number", "unit": "s", "label": "Worker : intervalle check classement mensuel", "default": "3600", "required": false, "description": "Frequence a laquelle le worker verifie s il faut publier le classement mensuel. L API ne publie qu au passage de mois, donc un tick horaire suffit."}]');
INSERT INTO public.bot_definitions VALUES ('announcements', 'Annonces planifiees', 'Messages Discord postes automatiquement (ponctuel, quotidien, hebdo, mensuel) avec embed riche, mentions, boutons interactifs et reactions automatiques. Le timer de publication tourne dans sentinel-worker.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Active la publication automatique des annonces planifiees pour ce serveur."}, {"key": "default_color_hex", "type": "text", "label": "Couleur par defaut (embed)", "default": "#5865f2", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Couleur d''accent pour les embeds, en hex (ex: #5865f2). Surchargeable par annonce."}, {"key": "max_announcements_per_guild", "max": 1000, "min": 1, "type": "number", "label": "Nombre max d''annonces par serveur", "default": "100", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "default_mention_everyone", "type": "boolean", "label": "Activer @everyone par defaut", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Si actif, les nouvelles annonces ont @everyone coche par defaut."}, {"key": "history_retention_days", "max": 365, "min": 7, "type": "number", "unit": "jours", "label": "Retention historique (jours)", "default": "90", "required": false, "depends_on": {"key": "enabled", "equals": "true"}}, {"key": "log_channel_id", "type": "channel", "label": "Salon de logs", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon ou poster les events publication / erreurs (optionnel)."}, {"key": "fetch_limit", "max": 500, "min": 1, "type": "number", "label": "Annonces fetchees par tick worker", "default": "50", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre max d''annonces traitees par le worker a chaque heure pile. Ne touche pas sauf si tu vois des delais de publication."}, {"key": "announcement_publish_interval_secs", "max": 86400, "min": 60, "type": "number", "unit": "s", "label": "Worker : intervalle publication annonces", "default": "3600", "required": false, "description": "Frequence a laquelle le worker publie les annonces dues sur la stream Redis. La boucle s aligne sur l heure pile au demarrage ; garder 3600 preserve l alignement HH:00."}]');
INSERT INTO public.bot_definitions VALUES ('guild-backup-bot', 'Sauvegarde serveur', 'Capture et restauration de la structure complete d un serveur (roles, salons, categories, parametres, bans, emojis). Les captures sont declenchees depuis le web ; le bot execute la capture/restauration sur Discord.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Active la sauvegarde/restauration du serveur."}, {"key": "snapshot_quota", "max": 100, "min": 1, "type": "number", "unit": "snapshots", "label": "Quota de sauvegardes", "default": "10", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre max de sauvegardes conservees par serveur (les plus anciennes sont evincees)."}, {"key": "auto_backup_enabled", "type": "boolean", "label": "Sauvegarde automatique", "default": "false", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Capture automatiquement le serveur a intervalle regulier."}, {"key": "auto_backup_interval_hours", "max": 168, "min": 1, "type": "number", "unit": "h", "label": "Intervalle de sauvegarde auto", "default": "24", "required": false, "depends_on": {"key": "auto_backup_enabled", "equals": "true"}, "description": "Delai entre deux captures automatiques."}, {"key": "restore_role_ids", "type": "text", "label": "Roles autorises a restaurer", "default": "", "required": false, "description": "IDs de roles autorises a declencher un restore (vide = Owner uniquement)."}]');

