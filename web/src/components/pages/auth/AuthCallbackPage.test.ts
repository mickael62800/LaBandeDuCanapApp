import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

import { HttpError } from "@/api/httpError";

// Mocks poses AVANT l'import du composant : il appelle `httpGet` des son
// montage, il n'y a pas de fenetre pour les installer apres.
const httpGet = vi.fn();
vi.mock("@/api/http", () => ({
  httpGet: (...args: unknown[]) => httpGet(...args),
  tryRefreshSession: vi.fn(),
  logoutSession: vi.fn(),
}));

const replace = vi.fn();
vi.mock("vue-router", () => ({
  useRouter: () => ({ replace }),
}));

import AuthCallbackPage from "./AuthCallbackPage.vue";
import { getDiscordToken, getDiscordUser } from "@/api/config";
import { useAuthStore } from "@/stores/authStore";

/// Le backend renvoie l'identite dans le FRAGMENT de l'URL.
function poserLeFragment() {
  window.location.hash = "#token=jeton-discord&id=42&username=admin&is_superadmin=1";
}

describe("AuthCallbackPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
    sessionStorage.clear();
    httpGet.mockReset();
    replace.mockReset();
    poserLeFragment();
  });

  it("ouvre la session quand check-access repond 200", async () => {
    httpGet.mockResolvedValue({});

    mount(AuthCallbackPage);
    await flushPromises();

    expect(getDiscordUser()?.id).toBe("42");
    expect(useAuthStore().user?.id).toBe("42");
    expect(replace).toHaveBeenCalled();
  });

  // Le coeur de W3 : une panne reseau ou un 5xx laissait passer, avec un simple
  // `console.warn`. L'interface d'administration devenait navigable alors que
  // les droits n'avaient jamais ete confirmes.
  it("n'ouvre PAS la session quand check-access echoue autrement qu'en 403", async () => {
    httpGet.mockRejectedValue(new HttpError("Service indisponible", { status: 503 }));

    const wrapper = mount(AuthCallbackPage);
    await flushPromises();

    expect(useAuthStore().user).toBeNull();
    // Rien ne doit ressembler a une session : `authStore.checkSession` relit
    // cette valeur au demarrage suivant.
    expect(getDiscordUser()).toBeNull();
    expect(replace).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("Vérification des accès impossible");
  });

  it("propose de reessayer sans refaire le tour OAuth", async () => {
    httpGet.mockRejectedValueOnce(new HttpError("Service indisponible", { status: 503 }));

    const wrapper = mount(AuthCallbackPage);
    await flushPromises();

    // Le jeton survit a l'echec : c'est lui qui rend la reprise possible.
    expect(getDiscordToken()).toBe("jeton-discord");

    httpGet.mockResolvedValueOnce({});
    await wrapper.find(".btn-retry").trigger("click");
    await flushPromises();

    expect(useAuthStore().user?.id).toBe("42");
    expect(replace).toHaveBeenCalled();
  });

  it("refuse et purge la session locale sur 403", async () => {
    vi.useFakeTimers();
    httpGet.mockRejectedValue(new HttpError("Interdit", { status: 403 }));

    const wrapper = mount(AuthCallbackPage);
    await flushPromises();

    expect(wrapper.text()).toContain("Accès refusé");
    expect(getDiscordToken()).toBe("");
    expect(getDiscordUser()).toBeNull();
    expect(useAuthStore().user).toBeNull();

    await vi.advanceTimersByTimeAsync(2600);
    expect(replace).toHaveBeenCalledWith({
      name: "login",
      query: { error: "not_authorized" },
    });
    vi.useRealTimers();
  });
});
