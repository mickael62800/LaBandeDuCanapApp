import { describe, expect, it, vi } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";

vi.mock("@/services/dockerService", () => ({
  dockerService: new Proxy({}, { get: (_t, prop) => (prop === Symbol.toPrimitive ? () => "" : vi.fn().mockResolvedValue({})) }),
}));
const successToast = vi.fn();
const errorToast = vi.fn();
vi.mock("@/composables/useToast", () => ({ useToast: () => ({ success: (...a) => successToast(...a), error: (...a) => errorToast(...a) }) }));
const confirmMock = vi.fn().mockResolvedValue(true);
vi.mock("@/composables/useConfirm", () => ({ useConfirm: () => ({ confirm: (...a) => confirmMock(...a) }) }));

import DockerAdminSection from "./DockerAdminSection.vue";

describe("debug prune DOM order", () => {
  it("logs buttons per card", async () => {
    const w = mount(DockerAdminSection);
    await flushPromises();
    // aller sur l'onglet Nettoyage
    const tabs = w.findAll(".tabs button");
    const t = tabs.find((x) => x.text().includes("Nettoy"));
    expect(t).toBeTruthy();
    await t!.trigger("click");
    await flushPromises();

    const cards = w.findAll(".prune-card");
    console.log("NB CARDS:", cards.length);
    cards.forEach((c, i) => {
      const btns = c.findAll("button").map((b) => JSON.stringify(b.text()));
      console.log(`CARD ${i}: [${btns.join(", ")}]`);
    });
  });
});
