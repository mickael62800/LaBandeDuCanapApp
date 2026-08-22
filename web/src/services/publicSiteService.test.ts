import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ publicGet: vi.fn() }));

vi.mock("./publicHttp", () => mocks);

import { guildIconUrl, publicSiteService, type PublicGuild } from "./publicSiteService";

describe("publicSiteService (site communautaire public)", () => {
  beforeEach(() => {
    mocks.publicGet.mockReset().mockResolvedValue({});
  });

  it("guild lit la vitrine du serveur via /api/public/guilds/{id} encode", async () => {
    const guilde = { guild_id: "g1", name: "La Bande Du Canap", member_count: 42 };
    mocks.publicGet.mockResolvedValue(guilde);

    await expect(publicSiteService.guild("g/9")).resolves.toBe(guilde);
    expect(mocks.publicGet).toHaveBeenCalledWith("/guilds/g%2F9");
  });

  it("propage les erreurs du transport public", async () => {
    const erreur = new Error("404");
    mocks.publicGet.mockRejectedValue(erreur);
    await expect(publicSiteService.guild("gX")).rejects.toBe(erreur);
  });
});

describe("guildIconUrl (icone Discord)", () => {
  it("construit l'URL CDN quand une icone existe", () => {
    const g: PublicGuild = { guild_id: "1234567890", name: "x", icon: "abc.def", member_count: 1 };
    expect(guildIconUrl(g)).toBe("https://cdn.discordapp.com/icons/1234567890/abc.def.png?size=128");
  });

  it("renvoie null sans icone (null ou vide)", () => {
    const g: PublicGuild = { guild_id: "g", name: "x", icon: null, member_count: 0 };
    expect(guildIconUrl(g)).toBeNull();
    expect(guildIconUrl({ ...g, icon: "" })).toBeNull();
  });
});
