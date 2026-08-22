import { describe, expect, it } from "vitest";

import { registry } from "chart.js";

import { registerChartJs } from "./chartjs";

describe("registerChartJs", () => {
  it("enregistre les elements sans lever (idempotent)", () => {
    // Premier appel : enregistrement reel. Deuxieme : no-op grace au flag.
    expect(() => registerChartJs()).not.toThrow();
    expect(() => registerChartJs()).not.toThrow();

    // Un scale et un element utilises par les pages graphiques sont bien connus.
    expect(registry.getScale("category")).toBeTruthy();
    expect(registry.getElement("point")).toBeTruthy();
  });
});
