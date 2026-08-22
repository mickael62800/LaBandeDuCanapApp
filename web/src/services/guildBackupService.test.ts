import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPatch: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { guildBackupService } from "./guildBackupService";

describe("guildBackupService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("listSnapshots lit les summaries du serveur", async () => {
    const snapshots = [{ id: "s1" }];
    mocks.httpGet.mockResolvedValue(snapshots);

    await expect(guildBackupService.listSnapshots("g1")).resolves.toEqual(
      snapshots,
    );
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/guild-backup/g1/snapshots");
  });

  it("getSnapshot lit le snapshot complet par son id", async () => {
    const snapshot = { roles: [], channels: [] };
    mocks.httpGet.mockResolvedValue(snapshot);

    await expect(guildBackupService.getSnapshot("s9")).resolves.toEqual(
      snapshot,
    );
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/guild-backup/snapshots/s9");
  });

  it("importSnapshot envoie le JSON dans la guild cible", async () => {
    const corps = { roles: [{ id: "r1" }] };
    mocks.httpPost.mockResolvedValue({ id: "s2" });

    await expect(guildBackupService.importSnapshot("g7", corps)).resolves.toEqual(
      { id: "s2" },
    );
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/guild-backup/g7/snapshots", corps);
  });

  it("rename renomme le snapshot avec son libelle", async () => {
    mocks.httpPatch.mockResolvedValue(undefined);

    await guildBackupService.rename("s9", "avant migration");
    expect(mocks.httpPatch).toHaveBeenCalledWith("/api/guild-backup/snapshots/s9", {
      label: "avant migration",
    });
  });

  it("remove supprime le snapshot par son id", async () => {
    mocks.httpDelete.mockResolvedValue(undefined);

    await guildBackupService.remove("s9");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/guild-backup/snapshots/s9");
  });

  it("capture declenche une capture sans libelle par defaut", async () => {
    mocks.httpPost.mockResolvedValue(undefined);

    await guildBackupService.capture("g1");
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/guild-backup/g1/capture", {});
  });

  it("capture transmet le libelle quand il est fourni", async () => {
    mocks.httpPost.mockResolvedValue(undefined);

    await guildBackupService.capture("g1", "avant refonte");
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/guild-backup/g1/capture", {
      label: "avant refonte",
    });
  });

  it("restore declenche la restauration avec le drapeau wipe", async () => {
    mocks.httpPost.mockResolvedValue(undefined);

    await guildBackupService.restore("s9", true);
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/guild-backup/snapshots/s9/restore", {
      wipe: true,
    });
  });
});
