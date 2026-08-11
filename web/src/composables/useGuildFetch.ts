import { ref, watch, type Ref, type WatchSource } from "vue";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

/**
 * Variante guild-aware de useFetch. Le fetcher recoit l'id de guild courant
 * (ou null) et est appele a chaque changement de selection.
 */
export function useGuildFetch<T>(
  fetcher: (guildId: string | null, signal: AbortSignal) => Promise<T>,
  initialValue: T,
  options?: {
    guildScoped?: boolean;
    immediate?: boolean;
    watchSources?: WatchSource[];
    label?: string;
  },
): {
  data: Ref<T>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
  refresh: () => Promise<void>;
} {
  const { error: showError } = useToast();
  const data = ref<T>(initialValue) as Ref<T>;
  const loading = ref(true);
  const error = ref<string | null>(null);
  const { guildIdFilter } = useGuildSelector();
  const label = options?.label ?? "donnees";

  const guildScoped = options?.guildScoped ?? true;
  const immediate = options?.immediate ?? true;
  let sequence = 0;
  let controller: AbortController | null = null;

  async function refresh() {
    const currentSequence = ++sequence;
    controller?.abort();
    controller = new AbortController();
    loading.value = true;
    error.value = null;
    try {
      const guildId = guildScoped ? (guildIdFilter.value ?? null) : null;
      const result = await fetcher(guildId, controller.signal);
      if (currentSequence === sequence) data.value = result;
    } catch (e) {
      if (currentSequence !== sequence || controller.signal.aborted) return;
      const msg = String(e);
      if (msg.includes("Connection refused") || msg.includes("network") || msg.includes("connect")) {
        error.value = "Connexion au serveur impossible. Verifiez que l'API est demarree.";
      } else if (msg.includes("timeout") || msg.includes("Timeout")) {
        error.value = "Le serveur met trop de temps a repondre. Reessayez plus tard.";
      } else {
        error.value = "Erreur lors du chargement des donnees.";
      }
      console.error(`Echec du chargement ${label} :`, e);
      showError(error.value ?? `Echec du chargement ${label}.`);
    } finally {
      if (currentSequence === sequence) loading.value = false;
    }
  }

  // Important : pattern singleton-friendly. onMounted ne fonctionne que dans
  // un setup() ; or beaucoup de composables hissent useGuildFetch au scope
  // module pour partager le cache entre organisms. Dans ce cas onMounted ne
  // fire jamais et la page reste sur "Chargement…" jusqu'a un F5.
  // Solution : si guildScoped, watch immediate fait office d'auto-fetch
  // (declenche au 1er load + a chaque changement de guild).
  if (guildScoped) {
    watch(guildIdFilter, refresh, { immediate });
  } else if (immediate) {
    void refresh();
  }
  if (options?.watchSources?.length) watch(options.watchSources, refresh);

  return { data, loading, error, refresh };
}
