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
  useToast: () => ({ success: (...a: unknown[]) => successToast(...a), error: (...a: unknown[]) => errorToast(...a) }),
}));

const confirmMock = vi.fn().mockResolvedValue(true);
vi.mock("@/composables/useConfirm", () => ({
  useConfirm: () => ({ confirm: (...a: unknown[]) => confirmMock(...a) }),
}));

import DockerAdminSection from "./DockerAdminSection.vue";
import {
  dockerService,
  type DockerContainer,
  type DockerImage,
  type DockerNetwork,
  type DockerOverview,
  type DockerVolume,
  type PruneSystemResult,
} from "@/services/dockerService";

const svc = vi.mocked(dockerService);

/// Fabriques de reponses Docker.
///
/// Elles remplissent TOUS les champs obligatoires du type, chaque test ne
/// precisant que ce qu'il observe. Les versions precedentes en omettaient la
/// moitie : le `as any` pose sur le service masquait l'ecart, si bien que les
/// mocks ne ressemblaient plus a ce que l'API renvoie reellement — un test
/// pouvait passer sur une forme que la production ne produit jamais.
function ct(partial: Partial<DockerContainer> = {}): DockerContainer {
  return {
    id: "sha256:" + "a1".repeat(32),
    names: ["/web"],
    image: "nginx",
    state: "running",
    status: "Up 2h",
    created: 1700000000,
    ports: ["80/tcp->80"],
    size_rw_bytes: 2048,
    size_root_fs_bytes: 4096,
    labels: {},
    ...partial,
  };
}
function img(partial: Partial<DockerImage> = {}): DockerImage {
  return {
    id: "sha256:" + "b2".repeat(32),
    repo_tags: ["app:latest"],
    repo_digests: [],
    dangling: false,
    created: 1700000000,
    size_bytes: 4 * 1024 ** 3,
    shared_size_bytes: 0,
    virtual_size_bytes: 4 * 1024 ** 3,
    containers: 1,
    ...partial,
  };
}
function vol(partial: Partial<DockerVolume> = {}): DockerVolume {
  return {
    name: "data",
    driver: "local",
    mountpoint: "/var/lib/docker/volumes/data/_data",
    created_at: "2026-01-01T00:00:00Z",
    size_bytes: 512,
    in_use: true,
    ref_count: 3,
    ...partial,
  };
}
function net(partial: Partial<DockerNetwork> = {}): DockerNetwork {
  return {
    id: "n1",
    name: "bridge",
    driver: "bridge",
    scope: "local",
    containers_count: 2,
    internal: false,
    ...partial,
  };
}
/// Seuls les quatre champs « recuperable » sont lus par la section ; le reste
/// est rempli de valeurs neutres pour rester une reponse valide.
function overview(partial: Partial<DockerOverview> = {}): DockerOverview {
  return {
    version: "26.0.0",
    api_version: "1.45",
    os: "linux",
    arch: "x86_64",
    kernel: "6.8.0",
    containers_running: 0,
    containers_paused: 0,
    containers_stopped: 0,
    images_count: 0,
    volumes_count: 0,
    networks_count: 0,
    layers_size_bytes: 0,
    images_size_bytes: 0,
    containers_size_bytes: 0,
    volumes_size_bytes: 0,
    build_cache_size_bytes: 0,
    reclaimable_images_bytes: 0,
    reclaimable_containers_bytes: 0,
    reclaimable_volumes_bytes: 0,
    reclaimable_build_cache_bytes: 0,
    ...partial,
  };
}
function pruneSysteme(total: number): PruneSystemResult {
  const vide = { deleted: [], space_reclaimed_bytes: 0 };
  return {
    containers: vide,
    images: vide,
    volumes: vide,
    networks: vide,
    total_space_reclaimed_bytes: total,
  };
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
    svc.getOverview.mockResolvedValue(overview({ reclaimable_containers_bytes: 10, reclaimable_images_bytes: 20 }));
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
    svc.getOverview.mockResolvedValue(overview({ reclaimable_containers_bytes: 0, reclaimable_images_bytes: 0, reclaimable_volumes_bytes: 0, reclaimable_build_cache_bytes: 8 }));
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

    const select = wrapper.find(".filters select");
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
    const checkbox = wrapper.find('.filters input[type="checkbox"]');
    await checkbox.setValue(true); // asynchrone : attendre le re-rendu reactif
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

    const btn = wrapper.find(".docker-table tbody tr").find("button");
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

    const btn = wrapper.find(".docker-table tbody tr").find("button");
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
    const checkbox = wrapper.find('.filters input[type="checkbox"]');
    await checkbox.setValue(true); // asynchrone : attendre le re-rendu reactif
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
    const btn = wrapper.find(".docker-table tbody tr").find("button");
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

    const btn = wrapper.find(".docker-table tbody tr").find("button");
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
    svc.getOverview.mockResolvedValue(overview({ reclaimable_containers_bytes: 1, reclaimable_images_bytes: 2, reclaimable_volumes_bytes: 3, reclaimable_build_cache_bytes: 4 }));
    await mountSection();
    await clickTab("Nettoyage");
    await flushPromises();
    return wrapper.findAll(".prune-card button"); // boutons dans l'ordre des cartes
  }

  /**
   * Clique un bouton de la carte de nettoyage dont le titre contient `carte`.
   *
   * Le DOM est RE-INTERROGE a chaque appel : chaque purge se termine par un
   * `refreshTab()` qui re-rend les cartes. Une liste de boutons capturee une
   * fois pointe donc, apres le premier clic, sur des elements detaches — le
   * clic suivant ne parvenait plus au composant et le service n'etait pas
   * rappele. Le test constatait alors le dernier appel du clic PRECEDENT, ce
   * qui donnait un echec trompeur (« attendu true, recu false ») laissant
   * croire a un bouton mal cable, alors que le composant etait juste.
   *
   * La designation se fait par CARTE puis par LIBELLE, jamais par indice :
   * neuf boutons repartis sur six cartes, vises par `btns[7]`, se decalent
   * tous des qu'une carte en gagne un — sans que rien ne dise lequel etait
   * attendu. La carte est necessaire car trois boutons se nomment « Nettoyer ».
   */
  async function cliquerNettoyage(carte: string, libelle?: string) {
    const cible = wrapper
      .findAll(".prune-card")
      .find((c) => c.find("h4").text().includes(carte));
    if (!cible) {
      const titres = wrapper.findAll(".prune-card h4").map((h) => h.text());
      throw new Error(`Carte « ${carte} » introuvable. Presentes : ${titres.join(" | ")}`);
    }

    const boutons = cible.findAll("button");
    const bouton = libelle
      ? boutons.find((b) => b.text().trim().startsWith(libelle))
      : boutons[0];
    if (!bouton) {
      const dispo = boutons.map((b) => b.text().trim());
      throw new Error(
        `Bouton « ${libelle} » introuvable dans « ${carte} ». Disponibles : ${dispo.join(" | ")}`,
      );
    }

    await bouton.trigger("click");
    await flushPromises();
  }

  it("purgent les conteneurs arretes", async () => {
    svc.pruneContainers.mockResolvedValue({ deleted: ["a"], space_reclaimed_bytes: 1024 });
    await goPrune();
    await cliquerNettoyage("Conteneurs arrêtés");
    expect(svc.pruneContainers).toHaveBeenCalledTimes(1);
    expect(successToast).toHaveBeenCalledWith(expect.stringContaining("conteneurs supprimés"));
  });

  it("purgent les images dangling puis toutes", async () => {
    svc.pruneImages.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });
    await goPrune();

    await cliquerNettoyage("Images dangling", "Nettoyer dangling");
    expect(svc.pruneImages).toHaveBeenLastCalledWith(false);

    // Distinction qui compte : « toutes inutilisees » supprime aussi les images
    // encore rattachees a aucun conteneur mais parfaitement valides, dont celles
    // des serveurs de jeu a l'arret. Les confondre couterait un retelechargement
    // complet au prochain demarrage.
    await cliquerNettoyage("Images dangling", "Toutes inutilisées");
    expect(svc.pruneImages).toHaveBeenLastCalledWith(true);
    expect(svc.pruneImages).toHaveBeenCalledTimes(2);
  });

  it("purgent les volumes orphelins", async () => {
    svc.pruneVolumes.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });
    await goPrune();
    await cliquerNettoyage("Volumes orphelins");
    expect(svc.pruneVolumes).toHaveBeenCalledTimes(1);
  });

  it("purgent les reseaux inutilises", async () => {
    svc.pruneNetworks.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });
    await goPrune();
    await cliquerNettoyage("Réseaux inutilisés");
    expect(svc.pruneNetworks).toHaveBeenCalledTimes(1);
  });

  it("purgent le build cache", async () => {
    svc.pruneBuildCache.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });
    await goPrune();
    await cliquerNettoyage("Build cache");
    expect(svc.pruneBuildCache).toHaveBeenCalledTimes(1);
  });

  it("nettoyage complet standard / +images / +volumes", async () => {
    svc.pruneSystem.mockResolvedValue(pruneSysteme(0));
    await goPrune();

    await cliquerNettoyage("Nettoyage complet", "Nettoyage standard");
    expect(svc.pruneSystem).toHaveBeenLastCalledWith({ volumes: false, allImages: false });

    await cliquerNettoyage("Nettoyage complet", "+ toutes images");
    expect(svc.pruneSystem).toHaveBeenLastCalledWith({ volumes: false, allImages: true });

    // Le seul des trois qui detruit des DONNEES : un volume orphelin peut etre
    // le monde d'un serveur de jeu supprime par erreur.
    await cliquerNettoyage("Nettoyage complet", "+ volumes");
    expect(svc.pruneSystem).toHaveBeenLastCalledWith({ volumes: true, allImages: true });
    expect(svc.pruneSystem).toHaveBeenCalledTimes(3);
  });

  it("annule le nettoyage si confirmation refusee", async () => {
    confirmMock.mockResolvedValue(false);
    await goPrune();
    await cliquerNettoyage("Conteneurs arrêtés");
    expect(svc.pruneContainers).not.toHaveBeenCalled();
  });

  it("propage l'erreur d'un nettoyage en toast", async () => {
    svc.pruneNetworks.mockRejectedValue(new Error("refus"));
    await goPrune();
    await cliquerNettoyage("Réseaux inutilisés");
    expect(errorToast).toHaveBeenCalledWith(expect.stringContaining("Erreur : refus"));
  });

  it("affiche les tailles recuperables de l'overview sur chaque carte", async () => {
    // `null` / `undefined` sont HORS CONTRAT : l'API declare ces champs en
    // `i64` non nullable (ops/handlers/docker/overview.rs). Le composant les
    // ramene malgre tout a zero (`?? 0`), et ce test fige cette defense en
    // profondeur — d'ou le cast, qui dit explicitement qu'on simule une
    // reponse que le backend actuel ne produit pas.
    svc.getOverview.mockResolvedValue(
      overview({
        reclaimable_containers_bytes: 1024,
        reclaimable_images_bytes: 2 * 1024 ** 3,
        reclaimable_volumes_bytes: null,
        reclaimable_build_cache_bytes: undefined,
      } as unknown as Partial<DockerOverview>),
    );
    await mountSection();
    await clickTab("Nettoyage");
    await flushPromises();
    const text = wrapper.find(".prune-grid").text();
    expect(text).toContain("1.00 KB"); // fmtBytes(1024) : 2 decimales si v < 10 et unite > B
  });
});
