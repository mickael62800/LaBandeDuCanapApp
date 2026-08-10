// Univers applicatif courant (Sentinel / Nexus / Atrium).
//
// La ROUTE fait foi : `meta.universe` est la seule source. Un lien direct, un
// favori ou un rechargement donnent donc toujours une barre laterale coherente
// avec la page affichee, sans qu'aucun composant n'ait a analyser l'URL.
//
// Les pages publiques ne declarent pas d'univers : on conserve alors le
// dernier univers d'administration visite plutot que de retomber sur Sentinel,
// ce qui faisait auparavant « sortir » de Nexus en passant par /membre.
//
// L'univers n'est pas un droit — cf. `universes.ts`.

import { computed, ref, watch } from "vue";
import { useRoute } from "vue-router";
import {
  DEFAULT_UNIVERSE,
  isUniverseKey,
  UNIVERSES,
  UNIVERSE_ORDER,
  type UniverseKey,
} from "@/universes";

const STORAGE_KEY = "ds.universe";

function readStored(): UniverseKey {
  const raw = localStorage.getItem(STORAGE_KEY);
  return isUniverseKey(raw) ? raw : DEFAULT_UNIVERSE;
}

// Dernier univers d'ADMINISTRATION visite. Etat partage au niveau module :
// une seule valeur, pas de chargement asynchrone, donc pas de store Pinia.
const lastAdminUniverse = ref<UniverseKey>(readStored());

watch(lastAdminUniverse, (u) => localStorage.setItem(STORAGE_KEY, u));

export function useUniverse() {
  const route = useRoute();

  /// Univers declare par la route courante, s'il y en a un.
  const declared = computed<UniverseKey | null>(() => {
    const meta = route.meta.universe;
    return isUniverseKey(meta) ? meta : null;
  });

  // Arriver sur une route d'administration memorise son univers : c'est lui
  // que retrouvera la prochaine page publique, et la prochaine session.
  watch(
    declared,
    (u) => {
      if (u && u !== lastAdminUniverse.value) lastAdminUniverse.value = u;
    },
    { immediate: true },
  );

  const universe = computed<UniverseKey>(
    () => declared.value ?? lastAdminUniverse.value,
  );

  const definition = computed(() => UNIVERSES[universe.value]);

  /// Liste ordonnee pour la bascule d'univers.
  const universes = UNIVERSE_ORDER.map((k) => UNIVERSES[k]);

  // Le back-office n'a qu'un seul utilisateur possible (superadmin), qui voit
  // tout : les trois univers sont toujours accessibles. Conserve sous forme de
  // drapeau pour que le jour ou un RBAC plus fin arrive, un seul point change.
  const canSwitchUniverse = true;

  function setUniverse(u: UniverseKey) {
    lastAdminUniverse.value = u;
  }

  /// Page d'accueil de l'univers courant — ce que doit viser le logo.
  const homePath = computed(() => definition.value.home);

  return {
    universe,
    definition,
    universes,
    canSwitchUniverse,
    setUniverse,
    homePath,
  };
}
