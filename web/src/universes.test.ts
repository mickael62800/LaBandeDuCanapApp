import { describe, expect, it } from "vitest";
import { isUniverseKey } from "./universes";

describe("isUniverseKey", () => {
  it.each(["sentinel", "nexus", "atrium", "ops"])("accepte l'univers %s", (universe) => {
    expect(isUniverseKey(universe)).toBe(true);
  });

  it("refuse une valeur inconnue", () => {
    expect(isUniverseKey("unknown")).toBe(false);
  });
});
