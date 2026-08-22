import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  opsDelete: vi.fn(),
  opsGet: vi.fn(),
  opsPost: vi.fn(),
}));

vi.mock("@/api/opsHttp", () => mocks);

import { dockerService } from "./dockerService";

describe("dockerService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("getOverview lit l'etat global de Docker", async () => {
    const apercu = { version: "27.1" };
    mocks.opsGet.mockResolvedValue(apercu);

    await expect(dockerService.getOverview()).resolves.toEqual(apercu);
    expect(mocks.opsGet).toHaveBeenCalledWith("/docker/overview");
  });

  it("listContainers lit tous les conteneurs par defaut", async () => {
    const conteneurs = [{ id: "c1" }];
    mocks.opsGet.mockResolvedValue(conteneurs);

    await expect(dockerService.listContainers()).resolves.toEqual(
      conteneurs,
    );
    expect(mocks.opsGet).toHaveBeenCalledWith("/docker/containers?all=true");
  });

  it("listContainers peut se limiter aux conteneurs actifs", async () => {
    mocks.opsGet.mockResolvedValue([]);

    await dockerService.listContainers(false);
    expect(mocks.opsGet).toHaveBeenCalledWith("/docker/containers?all=false");
  });

  it("startContainer demarre le conteneur par son id", async () => {
    mocks.opsPost.mockResolvedValue(undefined);

    await dockerService.startContainer("c1");
    expect(mocks.opsPost).toHaveBeenCalledWith("/docker/containers/c1/start");
  });

  it("stopContainer arrete le conteneur avec un delai par defaut", async () => {
    mocks.opsPost.mockResolvedValue(undefined);

    await dockerService.stopContainer("c1");
    expect(mocks.opsPost).toHaveBeenCalledWith(
      "/docker/containers/c1/stop?timeout=10",
    );
  });

  it("restartContainer redemarre le conteneur avec un delai choisi", async () => {
    mocks.opsPost.mockResolvedValue(undefined);

    await dockerService.restartContainer("c1", 30);
    expect(mocks.opsPost).toHaveBeenCalledWith(
      "/docker/containers/c1/restart?timeout=30",
    );
  });

  it("removeContainer supprime le conteneur avec ses options", async () => {
    mocks.opsDelete.mockResolvedValue(undefined);

    await dockerService.removeContainer("c1", true, true);
    expect(mocks.opsDelete).toHaveBeenCalledWith(
      "/docker/containers/c1?force=true&volumes=true",
    );
  });

  it("containerLogs lit la queue de logs avec horodatage optionnel", async () => {
    const logs = { logs: "ligne" };
    mocks.opsGet.mockResolvedValue(logs);

    await expect(dockerService.containerLogs("c1")).resolves.toEqual(
      logs,
    );
    expect(mocks.opsGet).toHaveBeenCalledWith(
      "/docker/containers/c1/logs?tail=200&timestamps=false",
    );

    mocks.opsGet.mockClear();
    await dockerService.containerLogs("c1", 50, true);
    expect(mocks.opsGet).toHaveBeenCalledWith(
      "/docker/containers/c1/logs?tail=50&timestamps=true",
    );
  });

  it("listImages lit les images du daemon", async () => {
    const images = [{ id: "i1" }];
    mocks.opsGet.mockResolvedValue(images);

    await expect(dockerService.listImages()).resolves.toEqual(images);
    expect(mocks.opsGet).toHaveBeenCalledWith("/docker/images");
  });

  it("removeImage supprime une image en force si demande", async () => {
    mocks.opsDelete.mockResolvedValue(undefined);

    await dockerService.removeImage("i1", true);
    expect(mocks.opsDelete).toHaveBeenCalledWith("/docker/images/i1?force=true");
  });

  it("listVolumes lit les volumes du daemon", async () => {
    const volumes = [{ name: "v1" }];
    mocks.opsGet.mockResolvedValue(volumes);

    await expect(dockerService.listVolumes()).resolves.toEqual(
      volumes,
    );
    expect(mocks.opsGet).toHaveBeenCalledWith("/docker/volumes");
  });

  it("removeVolume supprime un volume en force si demande", async () => {
    mocks.opsDelete.mockResolvedValue(undefined);

    await dockerService.removeVolume("v1", true);
    expect(mocks.opsDelete).toHaveBeenCalledWith(
      "/docker/volumes/v1?force=true",
    );
  });

  it("listNetworks lit les reseaux du daemon", async () => {
    const reseaux = [{ name: "bridge" }];
    mocks.opsGet.mockResolvedValue(reseaux);

    await expect(dockerService.listNetworks()).resolves.toEqual(
      reseaux,
    );
    expect(mocks.opsGet).toHaveBeenCalledWith("/docker/networks");
  });

  it("pruneContainers nettoie les conteneurs arretes", async () => {
    const resultat = { deleted: ["c1"], space_reclaimed_bytes: 10 };
    mocks.opsPost.mockResolvedValue(resultat);

    await expect(dockerService.pruneContainers()).resolves.toEqual(
      resultat,
    );
    expect(mocks.opsPost).toHaveBeenCalledWith("/docker/prune/containers");
  });

  it("pruneImages nettoie les images orphelines par defaut", async () => {
    mocks.opsPost.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });

    await dockerService.pruneImages();
    expect(mocks.opsPost).toHaveBeenCalledWith("/docker/prune/images?all=false");
  });

  it("pruneVolumes nettoie les volumes inutilises", async () => {
    mocks.opsPost.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });

    await dockerService.pruneVolumes();
    expect(mocks.opsPost).toHaveBeenCalledWith("/docker/prune/volumes");
  });

  it("pruneNetworks nettoie les reseaux inutilises", async () => {
    mocks.opsPost.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });

    await dockerService.pruneNetworks();
    expect(mocks.opsPost).toHaveBeenCalledWith("/docker/prune/networks");
  });

  it("pruneBuildCache nettoie le cache de build par defaut en totalite", async () => {
    mocks.opsPost.mockResolvedValue({ deleted: [], space_reclaimed_bytes: 0 });

    await dockerService.pruneBuildCache();
    expect(mocks.opsPost).toHaveBeenCalledWith(
      "/docker/prune/build-cache?all=true",
    );
  });

  it("pruneSystem applique les options par defaut (sans volumes, images orphelines)", async () => {
    const resultat = { total_space_reclaimed_bytes: 0 };
    mocks.opsPost.mockResolvedValue(resultat);

    await expect(dockerService.pruneSystem()).resolves.toEqual(
      resultat,
    );
    expect(mocks.opsPost).toHaveBeenCalledWith(
      "/docker/prune/system?volumes=false&all_images=false",
    );
  });

  it("pruneSystem transmet les options fournies", async () => {
    mocks.opsPost.mockResolvedValue({ total_space_reclaimed_bytes: 5 });

    await dockerService.pruneSystem({ volumes: true, allImages: true });
    expect(mocks.opsPost).toHaveBeenCalledWith(
      "/docker/prune/system?volumes=true&all_images=true",
    );
  });
});
