import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  opsGet: vi.fn(),
  opsPatch: vi.fn(),
}));

vi.mock("@/api/opsHttp", () => mocks);

import { alertRulesService } from "./alertRulesService";

describe("alertRulesService (backend ops-api)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("list lit les regles d'alerte de la machine", async () => {
    const rules = [
      { id: "r1", label: "CPU", metric: "cpu", comparator: "gt" },
    ];
    mocks.opsGet.mockResolvedValue(rules);

    await expect(alertRulesService.list()).resolves.toEqual(rules);
    expect(mocks.opsGet).toHaveBeenCalledWith("/alert-rules");
  });

  it("update envoie le patch de la regle par identifiant", async () => {
    const updated = { id: "r1", enabled: false, threshold: null };
    mocks.opsPatch.mockResolvedValue(updated);

    await expect(
      alertRulesService.update("r1", { enabled: false }),
    ).resolves.toEqual(updated);
    expect(mocks.opsPatch).toHaveBeenCalledWith("/alert-rules/r1", {
      enabled: false,
    });
  });
});
