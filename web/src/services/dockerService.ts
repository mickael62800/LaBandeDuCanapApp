import { opsDelete, opsGet, opsPost } from "@/api/opsHttp";

export interface DockerOverview {
  version: string;
  api_version: string;
  os: string;
  arch: string;
  kernel: string;
  containers_running: number;
  containers_paused: number;
  containers_stopped: number;
  images_count: number;
  volumes_count: number;
  networks_count: number;
  layers_size_bytes: number;
  images_size_bytes: number;
  containers_size_bytes: number;
  volumes_size_bytes: number;
  build_cache_size_bytes: number;
  reclaimable_images_bytes: number;
  reclaimable_containers_bytes: number;
  reclaimable_volumes_bytes: number;
  reclaimable_build_cache_bytes: number;
}

export interface DockerContainer {
  id: string;
  names: string[];
  image: string;
  state: string;
  status: string;
  created: number;
  size_rw_bytes: number | null;
  size_root_fs_bytes: number | null;
  ports: string[];
  labels: Record<string, string>;
}

export interface DockerImage {
  id: string;
  repo_tags: string[];
  repo_digests: string[];
  created: number;
  size_bytes: number;
  shared_size_bytes: number;
  virtual_size_bytes: number;
  containers: number;
  dangling: boolean;
}

export interface DockerVolume {
  name: string;
  driver: string;
  mountpoint: string;
  created_at: string | null;
  size_bytes: number | null;
  ref_count: number | null;
  in_use: boolean;
}

export interface DockerNetwork {
  id: string;
  name: string;
  driver: string;
  scope: string;
  internal: boolean;
  containers_count: number;
}

export interface DockerLogs {
  logs: string;
}

export interface PruneResult {
  deleted: string[];
  space_reclaimed_bytes: number;
}

export interface PruneSystemResult {
  containers: PruneResult;
  images: PruneResult;
  volumes: PruneResult;
  networks: PruneResult;
  total_space_reclaimed_bytes: number;
}

export const dockerService = {
  getOverview(): Promise<DockerOverview> {
    return opsGet("/docker/overview");
  },
  listContainers(all = true): Promise<DockerContainer[]> {
    return opsGet(`/docker/containers?all=${all}`);
  },
  startContainer(id: string) {
    return opsPost(`/docker/containers/${id}/start`);
  },
  stopContainer(id: string, timeout = 10) {
    return opsPost(`/docker/containers/${id}/stop?timeout=${timeout}`);
  },
  restartContainer(id: string, timeout = 10) {
    return opsPost(`/docker/containers/${id}/restart?timeout=${timeout}`);
  },
  removeContainer(id: string, force = false, volumes = false) {
    return opsDelete(`/docker/containers/${id}?force=${force}&volumes=${volumes}`);
  },
  containerLogs(id: string, tail = 200, timestamps = false): Promise<DockerLogs> {
    return opsGet(`/docker/containers/${id}/logs?tail=${tail}&timestamps=${timestamps}`);
  },
  listImages(): Promise<DockerImage[]> {
    return opsGet("/docker/images");
  },
  removeImage(id: string, force = false) {
    return opsDelete(`/docker/images/${id}?force=${force}`);
  },
  listVolumes(): Promise<DockerVolume[]> {
    return opsGet("/docker/volumes");
  },
  removeVolume(name: string, force = false) {
    return opsDelete(`/docker/volumes/${name}?force=${force}`);
  },
  listNetworks(): Promise<DockerNetwork[]> {
    return opsGet("/docker/networks");
  },
  pruneContainers(): Promise<PruneResult> {
    return opsPost("/docker/prune/containers");
  },
  pruneImages(all = false): Promise<PruneResult> {
    return opsPost(`/docker/prune/images?all=${all}`);
  },
  pruneVolumes(): Promise<PruneResult> {
    return opsPost("/docker/prune/volumes");
  },
  pruneNetworks(): Promise<PruneResult> {
    return opsPost("/docker/prune/networks");
  },
  pruneBuildCache(all = true): Promise<PruneResult> {
    return opsPost(`/docker/prune/build-cache?all=${all}`);
  },
  pruneSystem(opts?: { volumes?: boolean; allImages?: boolean }): Promise<PruneSystemResult> {
    const v = opts?.volumes ?? false;
    const a = opts?.allImages ?? false;
    return opsPost(`/docker/prune/system?volumes=${v}&all_images=${a}`);
  },
};
