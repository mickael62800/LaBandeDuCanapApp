import { ref, onMounted, onUnmounted, type Ref } from "vue";
import { useToast } from "./useToast";

/**
 * Helper generique : prend une fonction service (ex: () => guildsService.getAll())
 * et expose le triplet { data, loading, error } + une action refresh().
 */
export function useFetch<T>(
  fetcher: (signal: AbortSignal) => Promise<T>,
  initialValue: T,
  label = "donnees",
): { data: Ref<T>; loading: Ref<boolean>; error: Ref<string | null>; refresh: () => Promise<void> } {
  const { error: showError } = useToast();
  const data = ref<T>(initialValue) as Ref<T>;
  const loading = ref(true);
  const error = ref<string | null>(null);
  let mounted = true;
  let sequence = 0;
  let controller: AbortController | null = null;

  async function refresh() {
    const currentSequence = ++sequence;
    controller?.abort();
    controller = new AbortController();
    loading.value = true;
    error.value = null;
    try {
      const result = await fetcher(controller.signal);
      if (mounted && currentSequence === sequence) data.value = result;
    } catch (e) {
      if (!mounted || currentSequence !== sequence || controller.signal.aborted) return;
      error.value = String(e);
      console.error(`Echec du chargement ${label} :`, e);
      showError(`Echec du chargement ${label}.`);
    } finally {
      if (mounted && currentSequence === sequence) loading.value = false;
    }
  }

  onMounted(refresh);
  onUnmounted(() => {
    mounted = false;
    controller?.abort();
  });

  return { data, loading, error, refresh };
}
