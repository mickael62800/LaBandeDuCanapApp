import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { botConfigService } from "./botConfigService";

describe("botConfigService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("getDefinitions lit le catalogue des bots connus", async () => {
    const defs = [{ name: "nexus" }];
    mocks.httpGet.mockResolvedValue(defs);

    await expect(botConfigService.getDefinitions()).resolves.toEqual(defs);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/bots/definitions");
  });

  it("getGuildConfig lit la configuration des bots du serveur", async () => {
    const configs = [{ bot_name: "nexus" }];
    mocks.httpGet.mockResolvedValue(configs);

    await expect(botConfigService.getGuildConfig("g1")).resolves.toEqual(
      configs,
    );
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/bots/config/g1");
  });

  it("set ecrit une cle de configuration d'un bot du serveur", async () => {
    mocks.httpPost.mockResolvedValue({ ok: true });

    await expect(
      botConfigService.set("g1", "nexus", "welcome_channel", "c9"),
    ).resolves.toEqual({ ok: true });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/bots/config", {
      guild_id: "g1",
      bot_name: "nexus",
      config_key: "welcome_channel",
      config_value: "c9",
    });
  });

  it("remove supprime une cle de configuration d'un bot du serveur", async () => {
    mocks.httpDelete.mockResolvedValue(undefined);

    await botConfigService.remove("g1", "nexus", "welcome_channel");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/bots/config", {
      guild_id: "g1",
      bot_name: "nexus",
      config_key: "welcome_channel",
    });
  });
});
