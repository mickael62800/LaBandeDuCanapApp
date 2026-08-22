import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  nexusGet: vi.fn(),
  nexusPut: vi.fn(),
}));

vi.mock("@/api/nexusHttp", () => mocks);

import { chancePercent, nexusWheelService } from "./nexusWheelService";

describe("nexusWheelService (Roue du Destin)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("list lit les cases de la guilde", async () => {
    const roue = { cases: [{ key: "k1", label: "+50", payout: 50, weight: 3 }], customized: false };
    mocks.nexusGet.mockResolvedValue(roue);

    await expect(nexusWheelService.list("g/1")).resolves.toBe(roue);
    expect(mocks.nexusGet).toHaveBeenCalledWith("/api/wheel/g%2F1/cases", "g/1");
  });

  it("replace ecrase integralement la roue (PUT + corps { cases })", async () => {
    const maj = { cases: [], customized: true };
    mocks.nexusPut.mockResolvedValue(maj);
    const cases = [
      { key: "a", label: "+10", payout: 10, weight: 2 },
      { key: "b", label: "-5", payout: -5, weight: 1 },
    ];

    await expect(nexusWheelService.replace("g9", cases)).resolves.toBe(maj);
    expect(mocks.nexusPut).toHaveBeenCalledWith("/api/wheel/g9/cases", "g9", { cases });
  });

  it("replace avec liste vide restaure la roue d'origine (contrat API)", async () => {
    const maj = { cases: [], customized: false };
    mocks.nexusPut.mockResolvedValue(maj);

    await expect(nexusWheelService.replace("g10", [])).resolves.toBe(maj);
    expect(mocks.nexusPut).toHaveBeenLastCalledWith("/api/wheel/g10/cases", "g10", { cases: [] });
  });

  it("propage les erreurs du client Nexus", async () => {
    const erreur = new Error("503");
    mocks.nexusGet.mockRejectedValue(erreur);
    await expect(nexusWheelService.list("gX")).rejects.toBe(erreur);
  });
});

describe("chancePercent (lecture utile d'un poids)", () => {
  it("convertit un poids en part du total", () => {
    const cases = [
      { key: "a", label: "", payout: 0, weight: 3 },
      { key: "b", label: "", payout: 0, weight: 7 },
    ];

    expect(chancePercent(cases, 3)).toBeCloseTo(30);
    expect(chancePercent(cases, 7)).toBeCloseTo(70);
  });

  it("ignore les poids negatifs (Math.max) dans le total", () => {
    const cases = [
      { key: "a", label: "", payout: 0, weight: -5 },
      { key: "b", label: "", payout: 0, weight: 10 },
    ];

    expect(chancePercent(cases, 10)).toBeCloseTo(100);
  });

  it("renvoie 0 quand le total est nul (roue vide)", () => {
    expect(chancePercent([], 5)).toBe(0);
    const cases = [{ key: "a", label: "", payout: 0, weight: -2 }];
    expect(chancePercent(cases, -2)).toBe(0);
  });
});
