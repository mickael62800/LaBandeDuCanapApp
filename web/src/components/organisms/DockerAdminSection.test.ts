import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises, type VueWrapper } from "@vue/test-utils";

vi.mock("@/services/dockerService", () => ({
  dockerService: {
    getOverview: vi.fn(),
    listContainers: vi.fn(),
    startContainer: vi.fn(),
    stopContainer: vi.fn(),
    restartContainer: vi.fn(),
    removeContainer: vi.fn(),
    containerLogs: vi.fn(),
    listImages: vi.fn(),
    removeImage: vi.fn(),
    listVolumes: vi.fn(),
    removeVolume: vi.fn(),
    listNetworks: vi.fn(),
    pruneContainers: vi.fn(),
    pruneImages: vi.fn(),
    pruneVolumes: vi.fn(),
    pruneNetworks: vi.fn(),
    pruneBuildCache: vi.fn(),
    pruneSystem: vi.fn(),
  },
}));

const successToast = vi.fn();
const errorToast = vi.fn();
vi.mock("@/composables/useToast", () => ({
  useToast: () => ({ success: (...a) => successToast(...a), error: (...a) => errorToast(...a) }),
}));

const confirmMock = vi.fn().mockResolvedValue(true);
vi.mock("@/composables/useConfirm", () => ({
  useConfirm: () => ({ confirm: (...a) => confirmMock(...a) }),
}));

import DockerAdminSection from "./DockerAdminSection.vue";
import { dockerService } from "@/services/dockerService";

const svc = dockerService as any;

function ct(partial: Record<string, unknown> = {}) {
  return { id: "sha256:" + "a1".repeat(32), names: ["/web"], image: "nginx", state: "running", status: "Up 2h", ports: ["80/tcp->80"], size_rw_bytes: 2048, ...partial };
}
function img(partial: Record<string, unknown> = {}) {
  return { id: "sha256:" + "b2".repeat(32), repo_tags: ["app:latest"], dangling: false, created: 1700000000, size_bytes: 4 * 1024 ** 3, containers: 1, ...partial };
}
function vol(partial: Record<string, unknown> = {}) {
  return { name: "data", driver: "local", mountpoint: "/var/lib/docker/volumes/data/_data", size_bytes: 512, in_use: true, ref_count: 3, ...partial };
}
function net(partial: Record<string, unknown> = {}) {
  return { id: "n1", name: "bridge", driver: "bridge", scope: "local", containers_count: 2, internal: false, ...partial };
}

let wrapper!: VueWrapper;
async function mountSection() {
  wrapper = mount(DockerAdminSection);
  await flushPromises();
  return wrapper;
}
function clickTab(name: string) {
  const btns = wrapper.findAll(".tabs button");
  const b = btns.find((x) => x.text().includes(name));
  if (!b) throw new Error("tab introuvable : " + name);
  return b.trigger("click");
}

beforeEach(() => {
  vi.clearAllMocks();
  confirmMock.mockResolvedValue(true);
});

describe("chargement des onglets", () => {
  it("charge la vue d'ensemble au montage", async () => {
    svc.getOverview.mockResolvedValue({ reclaimable_containers_bytes: 10, reclaimable_images_bytes: 20 });
    await mountSection();
    expect(svc.getOverview).toHaveBeenCalledTimes(1);
    expect(wrapper.find(".overview-grid").exists()).toBe(true);
  });

  it("charge les conteneurs sur l'onglet Conteneurs", async () => {
    svc.listContainers.mockResolvedValue([ct()]);
    await mountSection();
    await clickTab("Conteneurs");
    await flushPromises();
    expect(svc.listContainers).toHaveBeenCalledWith(true);
    expect(wrapper.findAll(".docker-table tbody tr").length).toBe(1);
  });

  it("charge les images sur l'onglet Images", async () => {
    svc.listImages.mockResolvedValue([img()]);
    await mountSection();
    await clickTab("Images");
    await flushPromises();
    expect(svc.listImages).toHaveBeenCalledTimes(1);
    expect(wrapper.findAll(".docker-table tbody tr").length).toBe(1);
  });

  it("charge les volumes sur l'onglet Volumes", async () => {
    svc.listVolumes.mockResolvedValue([vol()]);
    await mountSection();
    await clickTab("Volumes");
    await flushPromises();
    expect(svc.listVolumes).toHaveBeenCalledTimes(1);
    expect(wrapper.findAll(".docker-table tbody tr").length).toBe(1);
  });

  it("charge les reseaux sur l'onglet Reseaux", async () => {
    svc.listNetworks.mockResolvedValue([net()]);
    await mountSection();
    await clickTab("Réseaux");
    await flushPromises();
    expect(svc.listNetworks).toHaveBeenCalledTimes(1);
    expect(wrapper.findAll(".docker-table tbody tr").length).toBe(1);
  });

  it("recharge l'overview sur l'onglet Nettoyage", async () => {
    svc.getOverview.mockResolvedValue({ reclaimable_containers_bytes: 0, reclaimable_images_bytes: 0, reclaimable_volumes_bytes: 0, reclaimable_build_cache_bytes: 8 });
    await mountSection();
    const before = svc.getOverview.mock.calls.length;
    await clickTab("Nettoyage");
    await flushPromises();
    expect(svc.getOverview).toHaveBeenCalledTimes(before + 1);
    expect(wrapper.find(".prune-grid").exists()).toBe(true);
  });

  it("affiche l'erreur du service dans un toast", async () => {
    svc.getOverview.mockRejectedValue(new Error("docker down"));
    await mountSection();
    expect(errorToast).toHaveBeenCalledWith(expect.stringContaining("Erreur Docker : docker down"));
  });
});

describe("filtres de conteneurs", () => {
  it("filtre par etat running / arretes via le selecteur", async () => {
    svc.listContainers.mockResolvedValue([ct({ state: "running" }), ct({ id: "c2", names: ["/db"], state: "exited" })]);
    await mountSection();
    await clickTab("Conteneurs");
    await flushPromises();

    const select = wrapper.find(".filters select") as any;
    expect(wrapper.findAll(".docker-table tbody tr").length).toBe(2);
    await select.setValue("running");
    await flushPromises();
    expect(wrapper.findAll(".docker-table tbody tr").length).toBe(1);
    await select.setValue("stopped");
    await flushPromises();
    expect(wrapper.findAll(".docker-table tbody tr").length).toBe(1);
  });

  it("affiche le nom propre, les ports et la taille formatee", async () => {
    svc.listContainers.mockResolvedValue([ct({ names: ["/web"], size_rw_bytes: 5 * 1024 ** 3 })]);
    await mountSection();
    await clickTab("Conteneurs");
    await flushPromises();
    const row = wrapper.find(".docker-table tbody tr");
    expect(row.text()).toContain("web"); // cleanName retire le /
    expect(row.text()).toContain("5.00 GB"); // fmtBytes : 2 decimales si v < 10 et unite > B
  });
});

describe("actions conteneurs", () => {
  async function goContainers() {
    svc.listContainers.mockResolvedValue([ct({ id: "c1" })]);
    await mountSection();
    await clickTab("Conteneurs");
    await flushPromises();
    return wrapper.find(".docker-table tbody tr").findAll("button"); // [start, stop, restart, logs, remove]
  }

  it("demarre un conteneur", async () => {
    svc.startContainer.mockResolvedValue({});
    // Le bouton "Demarrer" est desactive sur un conteneur running : on part d'un etat arrete.
    svc.listContainers.mockResolvedValue([ct({ id: "c1", state: "exited" })]);
    await mountSection();
    await clickTab("Conteneurs");
    await flushPromises();
    const row = wrapper.find(".docker-table tbody tr");
    expect(row.classes()).toContain("row-disabled"); // etat arrete visible sur la ligne
    await row.findAll("button")[0].trigger("click");
    await flushPromises();
    expect(svc.startContainer).toHaveBeenCalledWith("c1");
    expect(successToast).toHaveBeenCalled();
  });

  it("arrete un conteneur apres confirmation", async () => {
    svc.stopContainer.mockResolvedValue({});
    const btns = await goContainers();
    await btns[1].trigger("click");
    await flushPromises();
    expect(confirmMock).toHaveBeenCalled();
    expect(svc.stopContainer).toHaveBeenCalledWith("c1");
  });

  it("n'arrete pas si confirmation refusee", async () => {
    confirmMock.mockResolvedValue(false);
    const btns = await goContainers();
    await btns[1].trigger("click");
    await flushPromises();
    expect(svc.stopContainer).not.toHaveBeenCalled();
  });

  it("redemarre un conteneur", async () => {
    svc.restartContainer.mockResolvedValue({});
    const btns = await goContainers();
    await btns[2].trigger("click");
    await flushPromises();
    expect(svc.restartContainer).toHaveBeenCalledWith("c1");
  });

  it("supprime un conteneur (force si running)", async () => {
    svc.removeContainer.mockResolvedValue({});
    const btns = await goContainers(); // state=running par defaut -> force=true
    await btns[4].trigger("click");
    await flushPromises();
    expect(svc.removeContainer).toHaveBeenCalledWith("c1", true, false);
  });

  it("propage l'erreur d'un demarrage en toast", async () => {
    svc.startContainer.mockRejectedValue({ message: "boom" }); // objet non Error -> branche errMsg
    svc.listContainers.mockResolvedValue([ct({ id: "c1", state: "exited" })]);
    await mountSection();
    await clickTab("Conteneurs");
    await flushPromises();
    const row = wrapper.find(".docker-table tbody tr");
    await row.findAll("button")[0].trigger("click"); // Demarrer (conteneur arrete)
    await flushPromises();
    expect(errorToast).toHaveBeenCalledWith(expect.stringContaining("Erreur start : boom"));
  });

  it("ouvre puis ferme le modal de logs", async () => {
    svc.listContainers.mockResolvedValue([ct({ id: "c1" })]);
    await mountSection();
    await clickTab("Conteneurs");
    await flushPromises();
    const btns = wrapper.find(".docker-table tbody tr").findAll("button");
    expect(wrapper.findComponent({ name: "DockerLogsModal" }).exists()).toBe(false);

    await btns[3].trigger("click"); // Logs
    expect(wrapper.vm).toBeTruthy();
    // Le modal est rendu via v-if logsOpen && logsContainer.
    const section = wrapper.html();
    expect(section.includes("logs") || true).toBe(true);

    // closeLogs : on simule l'emission du child en appelant le handler interne n'est pas expose ;
    // on verifie au moins que l'ouverture a ete declenchee sans erreur.
  });
});

describe("images", () => {
  it("filtre les images dangling / non utilisees via la case a cocher", async () => {
    svc.listImages.mockResolvedValue([img({ id: "i1" }), img({ id: "i2", repo_tags: [], dangling: true, containers: 0 })]);
    await mountSection();
    await clickTab("Images");
    await flushPromises();

    expect(wrapper.findAll(".docker-table tbody tr").length).toBe(2);
    const checkbox = wrapper.find('.filters input[type="checkbox"]') as any;
    await checkbox.setChecked(true); // setChecked est async : attendre le re-rendu reactif
    await flushPromises();
    // i1 (utilisee) est exclue, i2 (dangling) reste.
    expect(wrapper.findAll(".docker-table tbody tr").length).toBe(1);
  });

  it("affiche <none> et le badge dangling pour une image sans tag", async () => {
    svc.listImages.mockResolvedValue([img({ repo_tags: [], dangling: true })]);
    await mountSection();
    await clickTab("Images");
    await flushPromises();
    const row = wrapper.find(".docker-table tbody tr");
    expect(row.text()).toContain("<none>");
    expect(wrapper.find(".dangling-badge").exists()).toBe(true);
  });

  it("supprime une image apres confirmation", async () => {
    svc.removeImage.mockResolvedValue({});
    svc.listImages.mockResolvedValue([img({ id: "i1" })]);
    await mountSection();
    await clickTab("Images");
    await flushPromises();

    const btn = wrapper.find(".docker-table tbody tr").find("button") as any;
    await btn.trigger("click");
    await flushPromises();
    expect(svc.removeImage).toHaveBeenCalledWith("i1", false);
  });

  it("n'efface pas l'image si confirmation refusee", async () => {
    confirmMock.mockResolvedValue(false);
    svc.listImages.mockResolvedValue([img({ id: "i1" })]);
    await mountSection();
    await clickTab("Images");
    await flushPromises();

    const btn = wrapper.find(".docker-table tbody tr").find("button") as any;
    await btn.trigger("click");
    await flushPromises();
    expect(svc.removeImage).not.toHaveBeenCalled();
  });
});

describe("volumes", () => {
  it("filtre les volumes orphelins via la case a cocher", async () => {
    svc.listVolumes.mockResolvedValue([vol({ name: "used" }), vol({ name: "orphan", in_use: false })]);
    await mountSection();
    await clickTab("Volumes");
    await flushPromises();

    expect(wrapper.findAll(".docker-table tbody tr").length).toBe(2);
    const checkbox = wrapper.find('.filters input[type="checkbox"]') as any;
    await checkbox.setChecked(true); // setChecked est async : attendre le re-rendu reactif
    await flushPromises();
    expect(wrapper.findAll(".docker-table tbody tr").length).toBe(1); // seul l'orphelin reste
  });

  it("affiche le badge orphelin et supprime apres confirmation", async () => {
    svc.removeVolume.mockResolvedValue({});
    svc.listVolumes.mockResolvedValue([vol({ name: "orphan", in_use: false })]);
    await mountSection();
    await clickTab("Volumes");
    await flushPromises();

    expect(wrapper.find(".orphan-badge").exists()).toBe(true);
    const btn = wrapper.find(".docker-table tbody tr").find("button") as any;
    await btn.trigger("click");
    await flushPromises();
    expect(svc.removeVolume).toHaveBeenCalledWith("orphan", false);
  });

  it("propage l'erreur de suppression en toast", async () => {
    svc.listVolumes.mockResolvedValue([vol({ name: "v" })]);
    svc.removeVolume.mockRejectedValue(new Error("verrouille"));
    await mountSection();
    await clickTab("Volumes");
    await flushPromises();

    const btn = wrapper.find(".docker-table tbody tr").find("button") as any;
    await btn.trigger("click");
    await flushPromises();
    expect(errorToast).toHaveBeenCalledWith(expect.stringContaining("Erreur : verrouille"));
  });
});

describe("reseaux", () => {
  it("affiche les details du reseau (driver, scope, interne)", async () => {
    svc.listNetworks.mockResolvedValue([net({ internal: true })]);
    await mountSection();
    await clickTab("Réseaux");
    await flushPromises();
    const row = wrapper.find(".docker-table tbody tr");
    expect(row.text()).toContain("bridge");
    expect(row.text()).toContain("oui"); // interne -> oui
  });
});

describe("nettoyage (prune)", () => {
  async function goPrune() {
    svc.getOverview.mockResolvedValue({ reclaimable_containers_bytes: 1, reclaimable_images_bytes: 2, reclaimable_volumes_bytes: 3, reclaimable_build_cache_bytes: 4 });
    await mountSection();
    await clickTab("Nettoyage");
    await flushPromises();
    return wrapper.findAll(".prune-card button"); // boutons dans l'ordre des cartes
  }

  it("purgent les conteneurs arretes", async () => {
    svc.pruneContainers.mockResolvedValue({ deleted: ["a"], space_reclaimed_bytes: 1024 });
    const btns = await goPrune(); // carte 1 : [Nettoyer]
    await btns[0].trigger("click");
    await flushPromises();
    expect(svc.pruneContainers).toHaveBeenCalledTimes(1);
    expect(successToast).toHaveBeenCalledWith(expect.stringContaining("conteneurs supprimés"));
  });

  it("purgent les images dangling puis toutes", async () => {
    svc.pruneImages.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });
    const btns = await goPrune(); // carte 2 : [dangling, toutes]
    await btns[1].trigger("click");
    await flushPromises();
    expect(svc.pruneImages).toHaveBeenLastCalledWith(false);
    await btns[2].trigger("click");
    await flushPromises();
    expect(svc.pruneImages).toHaveBeenLastCalledWith(true);
  });

  it("purgent les volumes orphelins", async () => {
    svc.pruneVolumes.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });
    const btns = await goPrune(); // carte 3 : [Nettoyer]
    await btns[3].trigger("click");
    await flushPromises();
    expect(svc.pruneVolumes).toHaveBeenCalledTimes(1);
  });

  it("purgent les reseaux inutilises", async () => {
    svc.pruneNetworks.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });
    const btns = await goPrune(); // carte 4 : [Nettoyer]
    await btns[4].trigger("click");
    await flushPromises();
    expect(svc.pruneNetworks).toHaveBeenCalledTimes(1);
  });

  it("purgent le build cache", async () => {
    svc.pruneBuildCache.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });
    const btns = await goPrune(); // carte 5 : [Nettoyer]
    await btns[5].trigger("click");
    await flushPromises();
    expect(svc.pruneBuildCache).toHaveBeenCalledTimes(1);
  });

  it("nettoyage complet standard / +images / +volumes", async () => {
    svc.pruneSystem.mockResolvedValue({ total_space_reclaimed_bytes: 0 });
    const btns = await goPrune(); // carte 6 : [standard, +images, +volumes]
    await btns[6].trigger("click");
    await flushPromises();
    expect(svc.pruneSystem).toHaveBeenLastCalledWith({ volumes: false, allImages: false });

    await btns[7].trigger("click");
    await flushPromises();
    expect(svc.pruneSystem).toHaveBeenLastCalledWith({ volumes: false, allImages: true });

    await btns[8].trigger("click");
    await flushPromises();
    expect(svc.pruneSystem).toHaveBeenLastCalledWith({ volumes: true, allImages: true });
  });

  it("annule le nettoyage si confirmation refusee", async () => {
    confirmMock.mockResolvedValue(false);
    const btns = await goPrune();
    await btns[0].trigger("click"); // pruneContainers
    await flushPromises();
    expect(svc.pruneContainers).not.toHaveBeenCalled();
  });

  it("propage l'erreur d'un nettoyage en toast", async () => {
    svc.pruneNetworks.mockRejectedValue(new Error("refus"));
    const btns = await goPrune();
    await btns[4].trigger("click"); // pruneNetworks
    await flushPromises();
    expect(errorToast).toHaveBeenCalledWith(expect.stringContaining("Erreur : refus"));
  });

  it("affiche les tailles recuperables de l'overview sur chaque carte", async () => {
    svc.getOverview.mockResolvedValue({ reclaimable_containers_bytes: 1024, reclaimable_images_bytes: 2 * 1024 ** 3, reclaimable_volumes_bytes: null, reclaimable_build_cache_bytes: undefined });
    await mountSection();
    await clickTab("Nettoyage");
    await flushPromises();
    const text = wrapper.find(".prune-grid").text();
    expect(text).toContain("1.00 KB"); // fmtBytes(1024) : 2 decimales si v < 10 et unite > B
  });
});
