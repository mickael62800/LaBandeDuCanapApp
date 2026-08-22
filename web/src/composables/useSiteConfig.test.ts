import { beforeEach, describe, expect, it, vi } from "vitest";

const fetchWithTimeout = vi.hoisted(() => vi.fn());
vi.mock("@/api/httpTransport", () => ({ fetchWithTimeout }));

describe("siteConfig (loadSiteConfig / siteConfig)", () => {
  beforeEach(async () => {
    // Chaque scénario repart d'un module FRAIS : `config` est un état de
    // module, et les scénarios se suivent dans l'ordre du fichier.
    vi.resetModules();
    fetchWithTimeout.mockReset();
  });

  async function charger() {
    const mod = await import("../siteConfig");
    return mod.loadSiteConfig().then((c) => ({ c, site: () => mod.siteConfig() }));
  }

  it("fetch en echec : config vide, pas d'exception", async () => {
    fetchWithTimeout.mockRejectedValue(new Error("offline"));
    const { c, site } = await charger();
    expect(c).toEqual({ guildId: "", discordInvite: "" });
    expect(site()).toEqual({ guildId: "", discordInvite: "" });
  });

  it("reponse non ok : config vide", async () => {
    fetchWithTimeout.mockResolvedValue({ ok: false, json: async () => ({}) });
    const { c } = await charger();
    expect(c).toEqual({ guildId: "", discordInvite: "" });
  });

  it("charge la config : invite validee, valeurs trimmees", async () => {
    fetchWithTimeout.mockResolvedValue({
      ok: true,
      json: async () => ({
        guild_id: "123456789012345678 ",
        discord_invite: "https://discord.gg/abc",
      }),
    });
    const { c } = await charger();
    expect(c).toEqual({ guildId: "123456789012345678", discordInvite: "https://discord.gg/abc" });

    // Appelle bien /site-config.json sans cache.
    const [url, opts] = fetchWithTimeout.mock.calls[0];
    expect(url).toBe("/site-config.json");
    expect(opts).toEqual({ cache: "no-store" });
  });

  it("invite invalide : rejetee (http, autre host), guild conservee", async () => {
    const cas = [
      "http://discord.gg/abc", // pas https
      "https://evil.example.com/x", // mauvais hote
      "pas une url du tout", // parse impossible
    ];
    for (const invite of cas) {
      fetchWithTimeout.mockResolvedValue({
        ok: true,
        json: async () => ({ guild_id: "42", discord_invite: invite }),
      });
      const { c } = await charger();
      expect(c.guildId).toBe("42");
      expect(c.discordInvite).toBe("");
    }

    // Invite vide -> reste vide.
    fetchWithTimeout.mockResolvedValue({ ok: true, json: async () => ({ guild_id: "7" }) });
    const { c } = await charger();
    expect(c.discordInvite).toBe("");
  });

  it("invite discord.com (www) acceptee", async () => {
    fetchWithTimeout.mockResolvedValue({
      ok: true,
      json: async () => ({ guild_id: "9", discord_invite: "https://discord.com/invites/x" }),
    });
    const { c } = await charger();
    expect(c.discordInvite).toBe("https://discord.com/invites/x");
  });

  it("reponse non ok APRES un succes : garde la derniere config validee", async () => {
    fetchWithTimeout.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ guild_id: "100", discord_invite: "https://discord.gg/ok" }),
    });
    const premier = await charger();
    expect(premier.c.guildId).toBe("100");

    // Re-import du module : l'etat de `config` est conserve (memorisee par vitest).
    fetchWithTimeout.mockResolvedValueOnce({ ok: false, json: async () => ({}) });
    const mod = await import("../siteConfig");
    const deuxieme = await mod.loadSiteConfig();
    expect(deuxieme.guildId).toBe("100"); // pas repasse a vide
  });
});
