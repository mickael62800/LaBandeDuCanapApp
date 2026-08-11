import { computed, onMounted, onUnmounted, ref } from "vue";
import { useAuth } from "@/composables/useAuth";
import { useConfirm } from "@/composables/useConfirm";
import { useToast } from "@/composables/useToast";
import { siteConfig } from "@/siteConfig";
import { isOngoing, publicEventsService, type PublicEvent } from "@/services/publicEventsService";
import { publicGamesService, type PublicGameServer } from "@/services/publicGamesService";
import { nexusGamesService } from "@/services/nexusGamesService";
import {
  communityActionsService,
  communityLifeService,
  type Anniversary,
  type Newcomer,
  type NewsItem,
  type Poll,
  type Presence,
  type PublicLfgPost,
  type Spotlight,
} from "@/services/communityLifeService";

const PRESENCE_REFRESH_MS = 20_000;
const PUBLIC_CALLS = 6;

/** État et orchestration de la page publique membre. */
export function useMemberHomePage() {
  const guildId = siteConfig().guildId
    || ((import.meta.env.VITE_PUBLIC_GUILD_ID as string | undefined) ?? "");
  const { user } = useAuth();
  const { confirm } = useConfirm();
  const { success, error: showError } = useToast();
  const hasAdminAccess = computed(() => user.value?.is_superadmin === true);

  const events = ref<PublicEvent[]>([]);
  const servers = ref<PublicGameServer[]>([]);
  const lfg = ref<PublicLfgPost[]>([]);
  const polls = ref<Poll[]>([]);
  const spotlight = ref<Spotlight | null>(null);
  const anniversaries = ref<Anniversary[]>([]);
  const newcomers = ref<Newcomer[]>([]);
  const news = ref<NewsItem[]>([]);
  const presence = ref<Presence>({ voice: [], voice_total: 0, text: [] });
  const failures = ref(0);
  const loadingEvents = ref(true);
  const loadingServers = ref(true);
  const loadingLfg = ref(true);
  const busyLfg = ref<string | null>(null);
  const lfgError = ref<string | null>(null);
  const busyVote = ref<string | null>(null);
  const busyReveal = ref<string | null>(null);
  let presenceTimer: number | undefined;

  const allFailed = computed(() => failures.value >= PUBLIC_CALLS);
  const playersOnline = computed(() =>
    servers.value.reduce((total, server) => total + (server.online ? server.player_count : 0), 0),
  );
  const serversOnline = computed(() => servers.value.filter((server) => server.online).length);
  const ongoing = computed(() => events.value.filter((event) => isOngoing(event)));
  const nextEvent = computed(() => {
    const now = new Date();
    return events.value
      .filter((event) => new Date(event.starts_at) > now)
      .sort((a, b) => a.starts_at.localeCompare(b.starts_at))[0] ?? null;
  });
  const upcoming = computed(() => {
    const now = new Date();
    return events.value
      .filter((event) => new Date(event.starts_at) > now)
      .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
      .slice(1, 5);
  });

  const failed = () => { failures.value += 1; };

  function loadPresence(): void {
    if (!guildId) return;
    communityLifeService.presence(guildId)
      .then((result) => { presence.value = result; })
      .catch(() => { presence.value = { voice: [], voice_total: 0, text: [] }; });
  }

  onMounted(() => {
    if (!guildId) {
      loadingEvents.value = false;
      loadingServers.value = false;
      loadingLfg.value = false;
      return;
    }

    const from = new Date();
    from.setDate(from.getDate() - 30);
    const to = new Date();
    to.setDate(to.getDate() + 60);

    publicEventsService.list(guildId, from, to)
      .then((result) => { events.value = result; })
      .catch(() => { failed(); events.value = []; })
      .finally(() => { loadingEvents.value = false; });
    publicGamesService.listServers(guildId)
      .then((result) => { servers.value = result; })
      .catch(() => { failed(); servers.value = []; })
      .finally(() => { loadingServers.value = false; });
    communityLifeService.lfg(guildId)
      .then((result) => { lfg.value = result; })
      .catch(() => { failed(); lfg.value = []; })
      .finally(() => { loadingLfg.value = false; });

    const pollRequest = user.value
      ? communityActionsService.myPolls(guildId)
      : communityLifeService.polls(guildId);
    pollRequest
      .then((result) => { polls.value = result.filter((poll) => poll.is_open).slice(0, 2); })
      .catch(() => { failed(); polls.value = []; });
    communityLifeService.spotlight(guildId)
      .then((result) => { spotlight.value = result; })
      .catch(() => { failed(); spotlight.value = null; });
    communityLifeService.pulse(guildId)
      .then((result) => {
        anniversaries.value = result.anniversaries;
        newcomers.value = result.newcomers;
      })
      .catch(() => { failed(); anniversaries.value = []; newcomers.value = []; });
    communityLifeService.news(guildId)
      .then((result) => { news.value = result; })
      .catch(() => { failed(); news.value = []; });

    loadPresence();
    presenceTimer = window.setInterval(loadPresence, PRESENCE_REFRESH_MS);
  });

  onUnmounted(() => {
    if (presenceTimer) window.clearInterval(presenceTimer);
  });

  async function joinLfg(id: string): Promise<void> {
    if (!user.value || !guildId) return;
    busyLfg.value = id;
    lfgError.value = null;
    try {
      await communityActionsService.joinLfg(id);
      lfg.value = await communityLifeService.lfg(guildId);
    } catch (error) {
      lfgError.value = error instanceof Error ? error.message : "Impossible de rejoindre.";
    } finally {
      busyLfg.value = null;
    }
  }

  async function vote(pollId: string, optionId: string): Promise<void> {
    if (!user.value) return;
    busyVote.value = pollId;
    try {
      const updated = await communityActionsService.vote(pollId, optionId);
      polls.value = polls.value.map((poll) => poll.id === pollId ? updated : poll);
    } catch {
      // Les résultats visibles restent inchangés si le vote est refusé.
    } finally {
      busyVote.value = null;
    }
  }

  async function revealServerAddress(server: PublicGameServer): Promise<void> {
    if (!hasAdminAccess.value || !user.value || !guildId || busyReveal.value) return;
    const accepted = await confirm({
      title: "Révéler l'adresse maintenant",
      message: `Révéler immédiatement l'adresse de « ${server.name} » à tous les membres ? Cette action avance la date prévue et mentionne le rôle du jeu s'il existe.`,
    });
    if (!accepted) return;

    busyReveal.value = server.id;
    try {
      await nexusGamesService.revealIp(guildId, server.id, user.value.id);
      servers.value = await publicGamesService.listServers(guildId);
      success(`Adresse de ${server.name} révélée.`);
    } catch (error) {
      showError(error instanceof Error ? error.message : "Révélation impossible.");
    } finally {
      busyReveal.value = null;
    }
  }

  return {
    guildId,
    user,
    hasAdminAccess,
    allFailed,
    events,
    servers,
    lfg,
    polls,
    spotlight,
    anniversaries,
    newcomers,
    news,
    presence,
    loadingEvents,
    loadingServers,
    loadingLfg,
    busyLfg,
    lfgError,
    busyVote,
    busyReveal,
    playersOnline,
    serversOnline,
    ongoing,
    nextEvent,
    upcoming,
    joinLfg,
    vote,
    revealServerAddress,
  };
}
