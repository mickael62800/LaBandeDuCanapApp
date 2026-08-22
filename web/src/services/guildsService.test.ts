import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ httpGet: vi.fn() }));

vi.mock("@/api/http", () => mocks);

import { guildsService } from "./guildsService";

describe("guildsService", () => {
  beforeEach(() => {
    mocks.httpGet.mockReset();
  });

  it("getAll lit la liste des serveurs", async () => {
    const guilds = [{ id: "g1" }];
    mocks.httpGet.mockResolvedValue(guilds);

    await expect(guildsService.getAll()).resolves.toEqual([{ id: "g1" }]);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/guilds");
  });

  it("getMembers lit les membres d'un serveur", async () => {
    const members = [{ user_id: "u2" }];
    mocks.httpGet.mockResolvedValue(members);

    await expect(guildsService.getMembers("g1")).resolves.toEqual([
      { user_id: "u2" },
    ]);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/guilds/g1/members");
  });

  it("getTextChannels lit les salons texte d'un serveur", async () => {
    const channels = [{ id: "c1", name: "general", position: 0 }];
    mocks.httpGet.mockResolvedValue(channels);

    await expect(guildsService.getTextChannels("g1")).resolves.toEqual([
      { id: "c1", name: "general", position: 0 },
    ]);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/guilds/g1/channels");
  });

  it("getEmojis lit les emojis d'un serveur", async () => {
    const emojis = [{ id: "e1", name: "ok", animated: false }];
    mocks.httpGet.mockResolvedValue(emojis);

    await expect(guildsService.getEmojis("g1")).resolves.toEqual([
      { id: "e1", name: "ok", animated: false },
    ]);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/guilds/g1/emojis");
  });
});
