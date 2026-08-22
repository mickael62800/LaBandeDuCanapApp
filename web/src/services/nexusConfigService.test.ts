import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  nexusGet: vi.fn(),
  nexusPut: vi.fn(),
}));

vi.mock("@/api/nexusHttp", () => mocks);

import { nexusConfigService } from "./nexusConfigService";

describe("nexusConfigService (config modules Nexus)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("getDefinitions lit les definitions des modules", async () => {
    const defs = [{ name: "economy" }];
    mocks.nexusGet.mockResolvedValue(defs);

    await expect(nexusConfigService.getDefinitions("g1")).resolves.toBe(defs);
    expect(mocks.nexusGet).toHaveBeenCalledWith("/api/bots/definitions", "g1");
  });

  it("getGuildConfig remonte la config plate au format ligne par ligne", async () => {
    mocks.nexusGet.mockResolvedValue({ max_coins: "500", enabled: "true" });

    const lignes = await nexusConfigService.getGuildConfig("g2", "economy");

    expect(lignes).toEqual([
      { guild_id: "g2", bot_name: "economy", config_key: "max_coins", config_value: "500" },
      { guild_id: "g2", bot_name: "economy", config_key: "enabled", config_value: "true" },
    ]);
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/config/g2/economy",
      "g2",
    );
  });

  it("getGuildConfig tolere une reponse vide (null/undefined) et encode le chemin", async () => {
    mocks.nexusGet.mockResolvedValue(null);
    await expect(nexusConfigService.getGuildConfig("g3", "bot")).resolves.toEqual([]);

    mocks.nexusGet.mockReset().mockResolvedValue({});
    await nexusConfigService.getGuildConfig("g/4", "mon bot");
    expect(mocks.nexusGet).toHaveBeenLastCalledWith(
      "/api/config/g%2F4/mon%20bot",
      "g/4",
    );
  });

  it("set enregistre une cle via PUT { key, value } (chemin encode)", async () => {
    await nexusConfigService.set("g5", "economy", "max_coins", "100");

    expect(mocks.nexusPut).toHaveBeenCalledWith(
      "/api/config/g5/economy",
      "g5",
      { key: "max_coins", value: "100" },
    );

    await nexusConfigService.set("g/6", "a&b", "k", "v");
    expect(mocks.nexusPut).toHaveBeenLastCalledWith(
      "/api/config/g%2F6/a%26b",
      "g/6",
      { key: "k", value: "v" },
    );
  });

  it("remove equivaut a une valeur vide (pas de DELETE cote Nexus)", async () => {
    await nexusConfigService.remove("g7", "economy", "max_coins");

    expect(mocks.nexusPut).toHaveBeenCalledWith(
      "/api/config/g7/economy",
      "g7",
      { key: "max_coins", value: "" },
    );
  });

  it("propage les erreurs du client Nexus", async () => {
    const erreur = new Error("500");
    mocks.nexusGet.mockRejectedValue(erreur);
    await expect(nexusConfigService.getDefinitions("gX")).rejects.toBe(erreur);
  });
});
