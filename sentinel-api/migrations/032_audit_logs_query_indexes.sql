-- Indexes alignes sur les lectures devenues frequentes depuis que les actions
-- de moderation ont audit_logs pour source de verite.
--
-- audit_logs est partitionnee par created_at. Creer ces index sur la table
-- parente cree/attache les index correspondants sur les partitions existantes
-- et les propage aux nouvelles partitions.

-- Historique et statistiques d'un membre modere. L'index est partiel pour ne
-- pas penaliser les nombreux evenements sans cible ni les evenements non-mod.
CREATE INDEX IF NOT EXISTS idx_audit_logs_mod_target_created
    ON public.audit_logs (guild_id, target_id, created_at DESC)
    WHERE target_id IS NOT NULL AND event_type LIKE 'mod_%';

-- Quotas, classement et activite recente d'un moderateur.
CREATE INDEX IF NOT EXISTS idx_audit_logs_mod_actor_created
    ON public.audit_logs (guild_id, actor_id, created_at DESC)
    WHERE actor_id IS NOT NULL AND event_type LIKE 'mod_%';

-- Timeline d'un salon vocal : filtre par channel + liste de types, puis ordre
-- chronologique. Aucun index historique ne commencait par channel_id.
CREATE INDEX IF NOT EXISTS idx_audit_logs_channel_type_created
    ON public.audit_logs (channel_id, event_type, created_at ASC)
    WHERE channel_id IS NOT NULL;

-- Pas d'index JSONB details->>'action_id' : aucun chemin de production ne le
-- lit encore. L'identifiant canonique est audit_logs.id, deja en tete de la
-- cle primaire (id, created_at). De meme, l'identifiant Discord a ete promu
-- dans discord_entry_id et couvre par l'index unique de la migration 031.
