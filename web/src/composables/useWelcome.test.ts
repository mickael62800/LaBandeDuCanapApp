import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const mocks = vi.hoisted(() => ({
  getConfig: vi.fn(),
  saveConfig: vi.fn(),
  publishRules: vi.fn(),
}));
vi.mock("@/services/welcomeService", () => ({ welcomeService: mocks }));

// useWelcome hisse son state au scope module (singleton) : il faut que Pinia soit
// actif AVANT le chargement du graphe de modules, sinon defineStore() sans pinia.
setActivePinia(createPinia());
const { useGuildSelectorStore } = await import("../stores/guildSelectorStore");
const { useWelcome } = await import("./useWelcome"); // singleton créé ICI (1x pour tout le fichier)

describe("useWelcome", () => {
  const store = useGuildSelectorStore(); // même instance pinia que le composable

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getConfig.mockResolvedValue(null); // pas de config par défaut
    localStorage.clear();
    store.selectedGuildId = null; // état initial connu : aucune guilde
  });

  it("sans guilde : aucun appel API ; save/publish sont des no-ops silencieux", async () => {
    const w = useWelcome();
    await vi.waitFor(() => expect(w.loading.value).toBe(false));

    expect(mocks.getConfig).not.toHaveBeenCalled(); // fetcher court-circuité sans guildId

    await w.saveConfig({} as never); // no-op (pas de throw)
    await w.publishRules();          // idem
    expect(mocks.saveConfig).not.toHaveBeenCalled();
    expect(mocks.publishRules).not.toHaveBeenCalled();
  });

  it("chargement : config venue du service", async () => {
    mocks.getConfig.mockResolvedValue({ enabled: true, channel_id: "c1" } as never);
    store.selectGuild("g-42"); // déclenche le watch immédiat de useGuildFetch (déjà créé au chargement)

    const w = useWelcome();
    await vi.waitFor(() => expect(w.config.value).not.toBeNull());
    expect(mocks.getConfig).toHaveBeenCalledWith("g-42");
    expect(w.config.value).toEqual({ enabled: true, channel_id: "c1" });
  });

  it("saveConfig : met à jour la config partagée + toast succès ; saving=true pendant l'appel", async () => {
    store.selectGuild("g-42");
    mocks.saveConfig.mockResolvedValue({ enabled: true } as never);

    const w = useWelcome();
    await vi.waitFor(() => expect(w.loading.value).toBe(false));

    const p = w.saveConfig({ enabled: true } as never); // non attendu : on inspecte l'état intermédiaire…
    expect(w.saving.value).toBe(true);                  // …avant la résolution de la promesse.

    await p;
    expect(mocks.saveConfig).toHaveBeenCalledWith("g-42", { enabled: true });
    expect(w.config.value).toEqual({ enabled: true });  // config partagée mise à jour…
    const { useToast } = await import("./useToast");
    expect(useToast().toasts.value.some((t) => t.type === "success")).toBe(true);
  });

  it("saveConfig en échec : console.error + toast erreur + re-throw", async () => {
    store.selectGuild("g-42");
    mocks.saveConfig.mockRejectedValue(new Error("503"));
    const spyErr = vi.spyOn(console, "error").mockImplementation(() => {});

    const w = useWelcome();
    await vi.waitFor(() => expect(w.loading.value).toBe(false));

    await expect(w.saveConfig({} as never)).rejects.toThrow("503"); // re-lève pour l'appelant
    expect(spyErr).toHaveBeenCalled();
    const { useToast } = await import("./useToast");
    expect(useToast().toasts.value.some((t) => t.type === "error")).toBe(true);
  });

  it("publishRules : succès -> toast ; échec -> console + toast + re-throw", async () => {
    store.selectGuild("g-42");
    mocks.publishRules.mockResolvedValue({ ok: true } as never);

    const w = useWelcome();
    await vi.waitFor(() => expect(w.loading.value).toBe(false));

    await w.publishRules(); // ne lève pas en succès
    expect(mocks.publishRules).toHaveBeenCalledWith("g-42");
    const { useToast } = await import("./useToast");
    expect(useToast().toasts.value.some((t) => t.type === "success")).toBe(true);

    mocks.publishRules.mockRejectedValueOnce(new Error("salon introuvable"));
    const spyErr = vi.spyOn(console, "error").mockImplementation(() => {});
    await expect(w.publishRules()).rejects.toThrow(); // échec re-lévé
    expect(spyErr).toHaveBeenCalled();
    expect(useToast().toasts.value.some((t) => t.type === "error" && /règlement/i.test(t.message))).toBe(true);
  });

  it("fetchConfig : re-demande la config", async () => {
    store.selectGuild("g-42");
    mocks.getConfig.mockResolvedValue({ enabled: false } as never);

    const w = useWelcome();
    await vi.waitFor(() => expect(w.loading.value).toBe(false));
    expect(mocks.getConfig).toHaveBeenCalledTimes(1); // watch immédiat au chargement du module

    mocks.getConfig.mockResolvedValueOnce({ enabled: true } as never);
    await w.fetchConfig();
    expect(mocks.getConfig).toHaveBeenCalledTimes(2);
    expect(w.config.value).toEqual({ enabled: true });
  });
});
