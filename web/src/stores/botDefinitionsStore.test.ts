import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const mocks = vi.hoisted(() => ({ getDefinitions: vi.fn() }));

vi.mock("@/services/botConfigService", () => ({
  botConfigService: { getDefinitions: mocks.getDefinitions },
}));

import type { BotDefinition } from "@/types";
import { useBotDefinitionsStore } from "./botDefinitionsStore";

const DEFS: BotDefinition[] = [
  {
    bot_name: "automod",
    display_name: "Automodération",
    description: "Filtre les messages",
    config_schema: [],
  },
];

describe("useBotDefinitionsStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mocks.getDefinitions.mockReset();
  });

  it("ensure charge la liste des definitions et marque le store comme charge", async () => {
    mocks.getDefinitions.mockResolvedValue(DEFS);
    const store = useBotDefinitionsStore();

    expect(store.loaded).toBe(false);
    await expect(store.ensure()).resolves.toEqual(DEFS);

    expect(mocks.getDefinitions).toHaveBeenCalledTimes(1);
    expect(store.definitions).toEqual(DEFS);
    expect(store.loaded).toBe(true);
    expect(store.loading).toBe(false);
  });

  it("ensure ne re-interroge pas le backend une fois charge", async () => {
    mocks.getDefinitions.mockResolvedValue(DEFS);
    const store = useBotDefinitionsStore();

    await store.ensure();
    await store.ensure();

    expect(mocks.getDefinitions).toHaveBeenCalledTimes(1);
  });

  it("deduplique les appels en parallele (un seul GET)", async () => {
    let release!: (defs: BotDefinition[]) => void;
    mocks.getDefinitions.mockReturnValue(
      new Promise((resolve) => {
        release = resolve;
      }),
    );
    const store = useBotDefinitionsStore();

    expect(store.loading).toBe(false);
    const p1 = store.ensure();
    const p2 = store.ensure(); // dedup : renvoie la meme promesse en cours

    release(DEFS);
    const [a, b] = await Promise.all([p1, p2]);
    expect(a).toEqual(b);
    expect(mocks.getDefinitions).toHaveBeenCalledTimes(1);
  });

  it("recharge apres invalidate()", async () => {
    mocks.getDefinitions.mockResolvedValueOnce([]).mockResolvedValueOnce(DEFS);
    const store = useBotDefinitionsStore();

    await store.ensure();
    expect(store.definitions).toEqual([]);

    store.invalidate();
    expect(store.loaded).toBe(false);

    await store.ensure();
    expect(mocks.getDefinitions).toHaveBeenCalledTimes(2);
    expect(store.definitions).toEqual(DEFS);
  });

  it("propage l'erreur et remet loading a false", async () => {
    const boom = new Error("backend down");
    mocks.getDefinitions.mockRejectedValue(boom);
    const store = useBotDefinitionsStore();

    await expect(store.ensure()).rejects.toBe(boom);

    // finally : le flag est remis, et un nouvel essai reste possible.
    expect(store.loading).toBe(false);
    expect(store.loaded).toBe(false);
    expect(store.definitions).toEqual([]);

    mocks.getDefinitions.mockResolvedValueOnce(DEFS);
    await expect(store.ensure()).resolves.toEqual(DEFS);
  });
});
