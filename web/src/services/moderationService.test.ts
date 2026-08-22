import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { moderationService } from "./moderationService";

describe("moderationService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("executeBan envoie le contexte complet et l'actionId optionnel", async () => {
    await moderationService.executeBan("g1", "u9", "spam");
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/moderation/execute-ban", {
      guild_id: "g1",
      user_id: "u9",
      reason: "spam",
      action_id: undefined,
    });

    await moderationService.executeBan("g1", "u9", "raid", "inf-42");
    expect(mocks.httpPost).toHaveBeenLastCalledWith(
      "/api/moderation/execute-ban",
      { guild_id: "g1", user_id: "u9", reason: "raid", action_id: "inf-42" },
    );

    // null est normalise en undefined (champ absent du corps).
    await moderationService.executeBan("g1", "u9", "spam", null);
    expect(mocks.httpPost).toHaveBeenLastCalledWith(
      "/api/moderation/execute-ban",
      { guild_id: "g1", user_id: "u9", reason: "spam", action_id: undefined },
    );
  });

  it("executeUnban envoie guilde et utilisateur", async () => {
    await moderationService.executeUnban("g2", "u5");
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/moderation/execute-unban", {
      guild_id: "g2",
      user_id: "u5",
    });
  });

  it("executeMute envoie duree et nom cible optionnels", async () => {
    await moderationService.executeMute("g3", "u7", "flood");
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/moderation/execute-mute", {
      guild_id: "g3",
      user_id: "u7",
      reason: "flood",
      duration: undefined,
      target_name: undefined,
    });

    await moderationService.executeMute("g3", "u7", "flood", 1800, "micka");
    expect(mocks.httpPost).toHaveBeenLastCalledWith(
      "/api/moderation/execute-mute",
      { guild_id: "g3", user_id: "u7", reason: "flood", duration: 1800, target_name: "micka" },
    );
  });

  it("getConfirmedBans filtre par guilde quand fournie", async () => {
    await moderationService.getConfirmedBans();
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/moderation/bans");

    await moderationService.getConfirmedBans(null);
    expect(mocks.httpGet).toHaveBeenLastCalledWith("/api/moderation/bans");

    await moderationService.getConfirmedBans("g8");
    expect(mocks.httpGet).toHaveBeenLastCalledWith(
      "/api/moderation/bans?guild_id=g8",
    );
  });

  it("getHistory interroge le parcours disciplinaire du couple guilde/utilisateur", async () => {
    const historique = { entries: [] };
    mocks.httpGet.mockResolvedValue(historique);

    await expect(moderationService.getHistory("g9", "u1")).resolves.toBe(
      historique,
    );
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/moderation/history/g9/u1");
  });

  it("logAction serialise tous les champs (optionnels inclus)", async () => {
    await moderationService.logAction({
      guildId: "g1",
      channelId: "c1",
      moderatorId: "m1",
      moderatorName: "Modo",
      targetId: "u2",
      targetName: "Cible",
      actionType: "warn",
      reason: "regles 3.2",
    });

    expect(mocks.httpPost).toHaveBeenCalledWith("/api/moderation/actions", {
      guild_id: "g1",
      channel_id: "c1",
      moderator_id: "m1",
      moderator_name: "Modo",
      target_id: "u2",
      target_name: "Cible",
      action_type: "warn",
      reason: "regles 3.2",
      gravity: undefined,
      duration: undefined,
    });

    await moderationService.logAction({
      guildId: "g1",
      channelId: "c1",
      moderatorId: "m1",
      moderatorName: "Modo",
      targetId: "u2",
      targetName: "Cible",
      actionType: "mute",
      reason: "flood",
      gravity: "high",
      duration: 3600,
    });

    expect(mocks.httpPost).toHaveBeenLastCalledWith("/api/moderation/actions", {
      guild_id: "g1",
      channel_id: "c1",
      moderator_id: "m1",
      moderator_name: "Modo",
      target_id: "u2",
      target_name: "Cible",
      action_type: "mute",
      reason: "flood",
      gravity: "high",
      duration: 3600,
    });
  });

  it("propage les erreurs du client HTTP", async () => {
    const erreur = new Error("500");
    mocks.httpPost.mockRejectedValue(erreur);
    await expect(moderationService.executeUnban("g1", "u1")).rejects.toBe(
      erreur,
    );
  });
});
