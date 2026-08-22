import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const mocks = vi.hoisted(() => ({ getGuildConfig: vi.fn() }));

vi.mock("@/services/botConfigService", () => ({
  botConfigService: { getGuildConfig: mocks.getGuildConfig },
}));

import type { BotGuildConfig } from "@/types";
import { useBotEnabledStatusStore } from "./botEnabledStatusStore";

function row(bot_name: string, config_key = "enabled", config_value = "true"): BotGuildConfig {
  return { guild_id: "g1", bot_name, config_key, config_value };
}

describe("useBotEnabledStatusStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mocks.getGuildConfig.mockReset();
  });

  it("load charge les configs de la guild et derive enabledMap (fail-closed)", async () => {
    mocks.getGuildConfig.mockResolvedValue([
      row("automod", "enabled", "True"), // parseBoolConfig : insensible a la casse
      row("welcome", "enabled", "no"), // valeur fausse -> disabled
      row("levels", "other_key", "x"), // pas une cle enabled -> ignoree dans le map
    ]);
    const store = useBotEnabledStatusStore();

    expect(store.isBotEnabled("automod")).toBe(false); // avant chargement : false
    await store.load("g1");

    expect(mocks.getGuildConfig).toHaveBeenCalledWith("g1");
    expect(store.configs).toHaveLength(3);
    expect(store.enabledMap).toEqual({ automod: true, welcome: false });
    expect(store.isBotEnabled("automod")).toBe(true);
    expect(store.isBotEnabled("welcome")).toBe(false);
    expect(store.disabledBots).toEqual(["welcome"]);
    expect(store.disabledCount).toBe(1);
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it("ne re-interroge pas le backend pour la meme guild deja chargee", async () => {
    mocks.getGuildConfig.mockResolvedValue([row("automod")]);
    const store = useBotEnabledStatusStore();

    await store.load("g1");
    await store.load("g1");

    expect(mocks.getGuildConfig).toHaveBeenCalledTimes(1);
  });

  it("deduplique les appels en parallele (un seul GET)", async () => {
    let release!: (rows: BotGuildConfig[]) => void;
    mocks.getGuildConfig.mockReturnValue(
      new Promise((resolve) => {
        release = resolve;
      }),
    );
    const store = useBotEnabledStatusStore();

    expect(store.loading).toBe(false);
    const p1 = store.load("g1");
    const p2 = store.load("g1"); // dedup : renvoie la meme promesse en cours

    release([row("automod")]);
    await Promise.all([p1, p2]);
    expect(mocks.getGuildConfig).toHaveBeenCalledTimes(1);
  });

  it("recharge quand la guild change", async () => {
    mocks.getGuildConfig.mockResolvedValue([]);
    const store = useBotEnabledStatusStore();

    await store.load("g1");
    await store.load("g2");

    expect(mocks.getGuildConfig).toHaveBeenCalledTimes(2);
  });

  it("recharge apres invalidate()", async () => {
    mocks.getGuildConfig.mockResolvedValue([]);
    const store = useBotEnabledStatusStore();

    await store.load("g1");
    store.invalidate();
    await store.load("g1");

    expect(mocks.getGuildConfig).toHaveBeenCalledTimes(2);
  });

  it("capture l'erreur, vide les configs et reste re-essayable", async () => {
    const boom = new Error("403 interdit");
    mocks.getGuildConfig.mockRejectedValueOnce(boom).mockResolvedValueOnce([row("automod")]);
    const store = useBotEnabledStatusStore();

    // load() avale l'exception (catch interne) : elle resout, et l'erreur est exposee.
    await expect(store.load("g1")).resolves.toBeUndefined();

    // finally : loading remis a false, erreur exposee.
    expect(store.loading).toBe(false);
    expect(store.error).toBe(String(boom));
    expect(store.configs).toEqual([]);
    expect(Object.keys(store.enabledMap)).toHaveLength(0);

    await store.load("g1");
    expect(mocks.getGuildConfig).toHaveBeenCalledTimes(2);
    expect(store.isBotEnabled("automod")).toBe(true);
  });

  it("reset vide les configs et force un rechargement", async () => {
    mocks.getGuildConfig.mockResolvedValue([row("automod")]);
    const store = useBotEnabledStatusStore();

    await store.load("g1");
    expect(store.configs).toHaveLength(1);

    store.reset();
    expect(store.configs).toEqual([]);
    expect(Object.keys(store.enabledMap)).toHaveLength(0);

    await store.load("g1");
    expect(mocks.getGuildConfig).toHaveBeenCalledTimes(2);
  });
});
