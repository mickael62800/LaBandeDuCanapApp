import { describe, it, vi } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";

vi.mock("@/services/dockerService", () => ({
  dockerService: {
    getOverview: vi.fn().mockResolvedValue({}), listContainers: vi.fn(), startContainer: vi.fn(), stopContainer: vi.fn(), restartContainer: vi.fn(), removeContainer: vi.fn(), containerLogs: vi.fn(),
    listImages: vi.fn(), removeImage: vi.fn(), listVolumes: vi.fn(), removeVolume: vi.fn(), listNetworks: vi.fn(),
    pruneContainers: vi.fn(), pruneImages: vi.fn(), pruneVolumes: vi.fn(), pruneNetworks: vi.fn(), pruneBuildCache: vi.fn(), pruneSystem: vi.fn(),
  },
}));
const successToast = vi.fn(); const errorToast = vi.fn();
vi.mock("@/composables/useToast", () => ({ useToast: () => ({ success: (...a) => successToast(...a), error: (...a) => errorToast(...a) }) }));
const confirmMock = vi.fn().mockResolvedValue(true);
vi.mock("@/composables/useConfirm", () => ({ useConfirm: () => ({ confirm: (...a) => confirmMock(...a) }) }));

import DockerAdminSection from "./DockerAdminSection.vue";
import { dockerService } from "@/services/dockerService";
const svc = dockerService as any;

describe("diag15 prune system", () => {
  it("clic btn6, btn7, btn8 : verifie les calls", async () => {
    svc.pruneSystem.mockResolvedValue({ total_space_reclaimed_bytes: 0 });
    svc.getOverview.mockResolvedValue({ reclaimable_containers_bytes: 1, reclaimable_images_bytes: 2, reclaimable_volumes_bytes: 3, reclaimable_build_cache_bytes: 4 });
    const w = mount(DockerAdminSection);
    await flushPromises();
    const btns = w.findAll(".tabs button");
    const b = btns.find((x) => x.text().includes("Nettoyage"))!;
    await b.trigger("click");
    await flushPromises();
    const buttons = w.findAll(".prune-card button");
    console.log("nb:", buttons.length, "btn6:", JSON.stringify(buttons[6].text()), "btn7:", JSON.stringify(buttons[7].text()), "btn8:", JSON.stringify(buttons[8].text()));

    await buttons[6].trigger("click");
    await flushPromises();
    console.log("APRES BTN6 pruneSystem:", JSON.stringify(svc.pruneSystem.mock.calls));

    await buttons[7].trigger("click");
    await flushPromises();
    console.log("APRES BTN7 pruneSystem:", JSON.stringify(svc.pruneSystem.mock.calls));

    await buttons[8].trigger("click");
    await flushPromises();
    console.log("APRES BTN8 pruneSystem:", JSON.stringify(svc.pruneSystem.mock.calls));
    console.log("successToast:", JSON.stringify(successToast.mock.calls), "errorToast:", JSON.stringify(errorToast.mock.calls));
  });
});
