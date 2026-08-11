<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
import AppSelect from "@/components/atoms/AppSelect.vue";
import DockerLogsModal from "./docker-admin/DockerLogsModal.vue";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { dockerService, type DockerContainer, type DockerImage, type DockerNetwork, type DockerOverview, type DockerVolume } from "@/services/dockerService";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";


const { success, error: showError } = useToast();
const { confirm } = useConfirm();

type Tab = "overview" | "containers" | "images" | "volumes" | "networks" | "prune";
const tab = ref<Tab>("overview");

const overview = ref<DockerOverview | null>(null);
const containers = ref<DockerContainer[]>([]);
const images = ref<DockerImage[]>([]);
const volumes = ref<DockerVolume[]>([]);
const networks = ref<DockerNetwork[]>([]);
const loading = ref(false);
const busy = ref(false);

const showOnlyDangling = ref(false);
const showOnlyUnused = ref(false);
const filterContainerState = ref<"all" | "running" | "stopped">("all");

// ── Logs modal ──
const logsOpen = ref(false);
const logsContainer = ref<DockerContainer | null>(null);

async function refreshTab() {
  loading.value = true;
  try {
    if (tab.value === "overview") overview.value = await dockerService.getOverview();
    else if (tab.value === "containers") containers.value = await dockerService.listContainers(true);
    else if (tab.value === "images") images.value = await dockerService.listImages();
    else if (tab.value === "volumes") volumes.value = await dockerService.listVolumes();
    else if (tab.value === "networks") networks.value = await dockerService.listNetworks();
    else if (tab.value === "prune") overview.value = await dockerService.getOverview();
  } catch (e: unknown) {
    console.error(e);
    showError(`Erreur Docker : ${errMsg(e)}`);
  } finally {
    loading.value = false;
  }
}

function setTab(t: Tab) {
  tab.value = t;
  refreshTab();
}

let pollHandle: number | null = null;
function startPoll() {
  if (pollHandle !== null) return;
  pollHandle = window.setInterval(refreshTab, 120_000);
}
function stopPoll() {
  if (pollHandle !== null) {
    clearInterval(pollHandle);
    pollHandle = null;
  }
}
onMounted(() => {
  refreshTab();
  startPoll();
});
onUnmounted(stopPoll);

// ── Helpers ──
/** Extrait un message d'erreur lisible depuis une valeur `unknown` (catch). */
function errMsg(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "object" && e !== null && "message" in e) {
    return String((e as { message: unknown }).message);
  }
  return String(e);
}
function fmtBytes(b: number | null | undefined): string {
  if (!b || b < 0) return "—";
  const u = ["B", "KB", "MB", "GB", "TB"];
  let v = b;
  let i = 0;
  while (v >= 1024 && i < u.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 && i > 0 ? 2 : 1)} ${u[i]}`;
}
function fmtTs(unix: number): string {
  if (!unix) return "—";
  return new Date(unix * 1000).toLocaleString("fr-FR");
}
function shortId(id: string): string {
  return id.replace(/^sha256:/, "").slice(0, 12);
}
function cleanName(n: string): string {
  return n.replace(/^\//, "");
}

const filteredContainers = computed(() => {
  if (filterContainerState.value === "running") return containers.value.filter((c) => c.state === "running");
  if (filterContainerState.value === "stopped") return containers.value.filter((c) => c.state !== "running");
  return containers.value;
});
const filteredImages = computed(() =>
  showOnlyDangling.value ? images.value.filter((i) => i.dangling || i.containers === 0) : images.value,
);
const filteredVolumes = computed(() =>
  showOnlyUnused.value ? volumes.value.filter((v) => !v.in_use) : volumes.value,
);

// ── Actions ──
async function doConfirm(msg: string): Promise<boolean> {
  return confirm({ title: "Confirmation", message: msg });
}

async function startCt(c: DockerContainer) {
  busy.value = true;
  try {
    await dockerService.startContainer(c.id);
    success(`Conteneur ${cleanName(c.names[0] ?? "")} démarré.`);
    await refreshTab();
  } catch (e: unknown) {
    showError(`Erreur start : ${errMsg(e)}`);
  } finally {
    busy.value = false;
  }
}
async function stopCt(c: DockerContainer) {
  if (!(await doConfirm(`Arrêter ${cleanName(c.names[0] ?? c.id)} ?`))) return;
  busy.value = true;
  try {
    await dockerService.stopContainer(c.id);
    success("Conteneur arrêté.");
    await refreshTab();
  } catch (e: unknown) {
    showError(`Erreur stop : ${errMsg(e)}`);
  } finally {
    busy.value = false;
  }
}
async function restartCt(c: DockerContainer) {
  busy.value = true;
  try {
    await dockerService.restartContainer(c.id);
    success("Conteneur redémarré.");
    await refreshTab();
  } catch (e: unknown) {
    showError(`Erreur restart : ${errMsg(e)}`);
  } finally {
    busy.value = false;
  }
}
async function removeCt(c: DockerContainer) {
  const force = c.state === "running";
  if (!(await doConfirm(`Supprimer ${cleanName(c.names[0] ?? c.id)} ?${force ? " (force)" : ""}`))) return;
  busy.value = true;
  try {
    await dockerService.removeContainer(c.id, force, false);
    success("Conteneur supprimé.");
    await refreshTab();
  } catch (e: unknown) {
    showError(`Erreur delete : ${errMsg(e)}`);
  } finally {
    busy.value = false;
  }
}

function openLogs(c: DockerContainer) {
  logsContainer.value = c;
  logsOpen.value = true;
}
function closeLogs() {
  logsOpen.value = false;
  logsContainer.value = null;
}

async function removeImg(img: DockerImage) {
  const tag = img.repo_tags[0] ?? shortId(img.id);
  if (!(await doConfirm(`Supprimer image ${tag} ?`))) return;
  busy.value = true;
  try {
    await dockerService.removeImage(img.id, false);
    success("Image supprimée.");
    await refreshTab();
  } catch (e: unknown) {
    showError(`Erreur : ${errMsg(e)}`);
  } finally {
    busy.value = false;
  }
}
async function removeVol(v: DockerVolume) {
  if (!(await doConfirm(`Supprimer volume ${v.name} ?`))) return;
  busy.value = true;
  try {
    await dockerService.removeVolume(v.name, false);
    success("Volume supprimé.");
    await refreshTab();
  } catch (e: unknown) {
    showError(`Erreur : ${errMsg(e)}`);
  } finally {
    busy.value = false;
  }
}

// ── Prune ──
async function pruneContainers() {
  if (!(await doConfirm("Supprimer tous les conteneurs arrêtés ?"))) return;
  busy.value = true;
  try {
    const r = await dockerService.pruneContainers();
    success(`${r.deleted.length} conteneurs supprimés · ${fmtBytes(r.space_reclaimed_bytes)} libérés.`);
    await refreshTab();
  } catch (e: unknown) { showError(`Erreur : ${errMsg(e)}`); } finally { busy.value = false; }
}
async function pruneImages(all: boolean) {
  const msg = all ? "Supprimer toutes les images non utilisées ?" : "Supprimer les images dangling ?";
  if (!(await doConfirm(msg))) return;
  busy.value = true;
  try {
    const r = await dockerService.pruneImages(all);
    success(`${r.deleted.length} images supprimées · ${fmtBytes(r.space_reclaimed_bytes)} libérés.`);
    await refreshTab();
  } catch (e: unknown) { showError(`Erreur : ${errMsg(e)}`); } finally { busy.value = false; }
}
async function pruneVolumes() {
  if (!(await doConfirm("⚠️ Supprimer tous les volumes orphelins ? Données potentiellement perdues."))) return;
  busy.value = true;
  try {
    const r = await dockerService.pruneVolumes();
    success(`${r.deleted.length} volumes supprimés · ${fmtBytes(r.space_reclaimed_bytes)} libérés.`);
    await refreshTab();
  } catch (e: unknown) { showError(`Erreur : ${errMsg(e)}`); } finally { busy.value = false; }
}
async function pruneNetworks() {
  if (!(await doConfirm("Supprimer les réseaux non utilisés ?"))) return;
  busy.value = true;
  try {
    const r = await dockerService.pruneNetworks();
    success(`${r.deleted.length} réseaux supprimés.`);
    await refreshTab();
  } catch (e: unknown) { showError(`Erreur : ${errMsg(e)}`); } finally { busy.value = false; }
}
async function pruneBuildCache() {
  const size = fmtBytes(overview.value?.reclaimable_build_cache_bytes ?? 0);
  if (!(await doConfirm(`Purger tout le build cache Docker (${size}) ? Les prochains builds seront plus lents.`))) return;
  busy.value = true;
  try {
    const r = await dockerService.pruneBuildCache();
    success(`Build cache purgé · ${fmtBytes(r.space_reclaimed_bytes)} libérés.`);
    await refreshTab();
  } catch (e: unknown) { showError(`Erreur : ${errMsg(e)}`); } finally { busy.value = false; }
}
async function pruneSystem(includeVolumes: boolean, allImages: boolean) {
  let msg = "Nettoyage système complet : conteneurs arrêtés + images";
  msg += allImages ? " (toutes inutilisées)" : " dangling";
  msg += " + réseaux";
  if (includeVolumes) msg += " + volumes orphelins ⚠️";
  msg += ". Continuer ?";
  if (!(await doConfirm(msg))) return;
  busy.value = true;
  try {
    const r = await dockerService.pruneSystem({ volumes: includeVolumes, allImages });
    success(`Nettoyage : ${fmtBytes(r.total_space_reclaimed_bytes)} libérés.`);
    await refreshTab();
  } catch (e: unknown) { showError(`Erreur : ${errMsg(e)}`); } finally { busy.value = false; }
}
</script>

<template>
  <section class="docker-section">
    <div class="docker-header">
      <h2 class="section-title">🐳 Docker</h2>
      <div class="tabs">
        <button :class="{ active: tab === 'overview' }" @click="setTab('overview')">Vue d'ensemble</button>
        <button :class="{ active: tab === 'containers' }" @click="setTab('containers')">Conteneurs</button>
        <button :class="{ active: tab === 'images' }" @click="setTab('images')">Images</button>
        <button :class="{ active: tab === 'volumes' }" @click="setTab('volumes')">Volumes</button>
        <button :class="{ active: tab === 'networks' }" @click="setTab('networks')">Réseaux</button>
        <button :class="{ active: tab === 'prune' }" @click="setTab('prune')">🧹 Nettoyage</button>
      </div>
    </div>

    <div v-if="loading" class="muted">Chargement…</div>

    <!-- ── Overview ── -->
    <div v-else-if="tab === 'overview' && overview" class="overview-grid">
      <div class="ov-card">
        <div class="ov-label">Version Docker</div>
        <div class="ov-value">{{ overview.version }}</div>
        <div class="ov-sub">API {{ overview.api_version }} · {{ overview.os }}/{{ overview.arch }}</div>
        <div class="ov-sub">Kernel : {{ overview.kernel }}</div>
      </div>
      <div class="ov-card">
        <div class="ov-label">Conteneurs</div>
        <div class="ov-value">{{ overview.containers_running }} / {{ overview.containers_running + overview.containers_paused + overview.containers_stopped }}</div>
        <div class="ov-sub">{{ overview.containers_running }} running · {{ overview.containers_paused }} paused · {{ overview.containers_stopped }} stopped</div>
        <div class="ov-sub">Taille writable : {{ fmtBytes(overview.containers_size_bytes) }}</div>
      </div>
      <div class="ov-card">
        <div class="ov-label">Images</div>
        <div class="ov-value">{{ overview.images_count }}</div>
        <div class="ov-sub">Taille totale : {{ fmtBytes(overview.images_size_bytes) }}</div>
        <div class="ov-sub reclaimable">Récupérables : {{ fmtBytes(overview.reclaimable_images_bytes) }}</div>
      </div>
      <div class="ov-card">
        <div class="ov-label">Volumes</div>
        <div class="ov-value">{{ overview.volumes_count }}</div>
        <div class="ov-sub">Taille totale : {{ fmtBytes(overview.volumes_size_bytes) }}</div>
        <div class="ov-sub reclaimable">Récupérables : {{ fmtBytes(overview.reclaimable_volumes_bytes) }}</div>
      </div>
      <div class="ov-card">
        <div class="ov-label">Build cache</div>
        <div class="ov-value">{{ fmtBytes(overview.build_cache_size_bytes) }}</div>
        <div class="ov-sub reclaimable">Récupérable : {{ fmtBytes(overview.reclaimable_build_cache_bytes) }}</div>
      </div>
      <div class="ov-card highlight">
        <div class="ov-label">Total récupérable</div>
        <div class="ov-value">{{ fmtBytes(overview.reclaimable_images_bytes + overview.reclaimable_containers_bytes + overview.reclaimable_volumes_bytes + overview.reclaimable_build_cache_bytes) }}</div>
        <div class="ov-sub">Lance un nettoyage pour libérer cet espace</div>
      </div>
    </div>

    <!-- ── Containers ── -->
    <div v-else-if="tab === 'containers'">
      <div class="filters">
        <AppSelect v-model="filterContainerState">
          <option value="all">Tous ({{ containers.length }})</option>
          <option value="running">Running ({{ containers.filter(c => c.state === 'running').length }})</option>
          <option value="stopped">Arrêtés ({{ containers.filter(c => c.state !== 'running').length }})</option>
        </AppSelect>
      </div>
      <table class="docker-table">
        <thead>
          <tr><th>Nom</th><th>Image</th><th>État</th><th>Statut</th><th>Ports</th><th>Taille</th><th class="actions-h">Actions</th></tr>
        </thead>
        <tbody>
          <tr v-for="c in filteredContainers" :key="c.id" :class="{ 'row-disabled': c.state !== 'running' }">
            <td><code>{{ cleanName(c.names[0] ?? shortId(c.id)) }}</code></td>
            <td class="muted">{{ c.image }}</td>
            <td><span class="state-pill" :class="c.state">{{ c.state }}</span></td>
            <td class="muted small">{{ c.status }}</td>
            <td class="ports small">{{ c.ports.join(', ') || '—' }}</td>
            <td class="small">{{ fmtBytes(c.size_rw_bytes ?? 0) }}</td>
            <td class="actions">
              <AppButton variant="ghost" size="xs" :disabled="busy || c.state === 'running'" title="Démarrer" @click="startCt(c)">▶</AppButton>
              <AppButton variant="ghost" size="xs" :disabled="busy || c.state !== 'running'" title="Arrêter" @click="stopCt(c)">⏹</AppButton>
              <AppButton variant="ghost" size="xs" :disabled="busy" title="Redémarrer" @click="restartCt(c)">↻</AppButton>
              <AppButton variant="ghost" size="xs" :disabled="busy" title="Logs" @click="openLogs(c)">📋</AppButton>
              <AppButton variant="danger" size="xs" :disabled="busy" title="Supprimer" @click="removeCt(c)">🗑</AppButton>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- ── Images ── -->
    <div v-else-if="tab === 'images'">
      <div class="filters">
        <label><input type="checkbox" v-model="showOnlyDangling" /> Uniquement non utilisées / dangling</label>
        <span class="muted">{{ filteredImages.length }} image(s)</span>
      </div>
      <table class="docker-table">
        <thead>
          <tr><th>Tag</th><th>ID</th><th>Créée</th><th>Taille</th><th>Conteneurs</th><th class="actions-h">Actions</th></tr>
        </thead>
        <tbody>
          <tr v-for="img in filteredImages" :key="img.id" :class="{ dangling: img.dangling }">
            <td>
              <code v-if="img.repo_tags.length > 0">{{ img.repo_tags[0] }}</code>
              <span v-else class="muted">&lt;none&gt;</span>
              <span v-if="img.dangling" class="badge dangling-badge">dangling</span>
            </td>
            <td class="small mono">{{ shortId(img.id) }}</td>
            <td class="small muted">{{ fmtTs(img.created) }}</td>
            <td class="small">{{ fmtBytes(img.size_bytes) }}</td>
            <td class="small">{{ img.containers > 0 ? img.containers : '—' }}</td>
            <td class="actions">
              <AppButton variant="danger" size="xs" :disabled="busy" title="Supprimer" @click="removeImg(img)">🗑</AppButton>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- ── Volumes ── -->
    <div v-else-if="tab === 'volumes'">
      <div class="filters">
        <label><input type="checkbox" v-model="showOnlyUnused" /> Uniquement orphelins</label>
        <span class="muted">{{ filteredVolumes.length }} volume(s)</span>
      </div>
      <table class="docker-table">
        <thead>
          <tr><th>Nom</th><th>Driver</th><th>Mountpoint</th><th>Taille</th><th>Réf</th><th class="actions-h">Actions</th></tr>
        </thead>
        <tbody>
          <tr v-for="v in filteredVolumes" :key="v.name" :class="{ orphan: !v.in_use }">
            <td><code>{{ v.name }}</code><span v-if="!v.in_use" class="badge orphan-badge">orphelin</span></td>
            <td class="small">{{ v.driver }}</td>
            <td class="small mono muted">{{ v.mountpoint }}</td>
            <td class="small">{{ fmtBytes(v.size_bytes) }}</td>
            <td class="small">{{ v.ref_count ?? '—' }}</td>
            <td class="actions">
              <AppButton variant="danger" size="xs" :disabled="busy" title="Supprimer" @click="removeVol(v)">🗑</AppButton>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- ── Networks ── -->
    <div v-else-if="tab === 'networks'">
      <table class="docker-table">
        <thead>
          <tr><th>Nom</th><th>Driver</th><th>Scope</th><th>Conteneurs</th><th>Interne</th></tr>
        </thead>
        <tbody>
          <tr v-for="n in networks" :key="n.id">
            <td><code>{{ n.name }}</code></td>
            <td class="small">{{ n.driver }}</td>
            <td class="small">{{ n.scope }}</td>
            <td class="small">{{ n.containers_count }}</td>
            <td class="small">{{ n.internal ? 'oui' : 'non' }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- ── Prune ── -->
    <div v-else-if="tab === 'prune'" class="prune-grid">
      <div class="prune-card">
        <h4>📦 Conteneurs arrêtés</h4>
        <p class="muted">Supprime tous les conteneurs en état non running.</p>
        <p v-if="overview" class="reclaim">Récupérable : {{ fmtBytes(overview.reclaimable_containers_bytes) }}</p>
        <AppButton variant="ghost" :disabled="busy" @click="pruneContainers">Nettoyer</AppButton>
      </div>
      <div class="prune-card">
        <h4>🖼 Images dangling</h4>
        <p class="muted">Images sans tag, jamais utilisées.</p>
        <p v-if="overview" class="reclaim">Récupérable : {{ fmtBytes(overview.reclaimable_images_bytes) }}</p>
        <AppButton variant="ghost" :disabled="busy" @click="pruneImages(false)">Nettoyer dangling</AppButton>
        <AppButton variant="warning" :disabled="busy" @click="pruneImages(true)">Toutes inutilisées</AppButton>
      </div>
      <div class="prune-card">
        <h4>💾 Volumes orphelins</h4>
        <p class="muted">⚠️ Volumes sans conteneur lié. Risque de perte de données.</p>
        <p v-if="overview" class="reclaim">Récupérable : {{ fmtBytes(overview.reclaimable_volumes_bytes) }}</p>
        <AppButton variant="danger" :disabled="busy" @click="pruneVolumes">Nettoyer</AppButton>
      </div>
      <div class="prune-card">
        <h4>🌐 Réseaux inutilisés</h4>
        <p class="muted">Réseaux sans conteneur attaché.</p>
        <AppButton variant="ghost" :disabled="busy" @click="pruneNetworks">Nettoyer</AppButton>
      </div>
      <div class="prune-card">
        <h4>🧱 Build cache</h4>
        <p class="muted">Cache de couches buildées non utilisées.</p>
        <p v-if="overview" class="reclaim">Récupérable : {{ fmtBytes(overview.reclaimable_build_cache_bytes) }}</p>
        <AppButton variant="warning" :disabled="busy" @click="pruneBuildCache">Nettoyer le build cache ({{ fmtBytes(overview?.reclaimable_build_cache_bytes ?? 0) }})</AppButton>
      </div>
      <div class="prune-card highlight">
        <h4>🚀 Nettoyage complet</h4>
        <p class="muted">conteneurs + images dangling + réseaux.</p>
        <AppButton variant="ghost" :disabled="busy" @click="pruneSystem(false, false)">Nettoyage standard</AppButton>
        <AppButton variant="warning" :disabled="busy" @click="pruneSystem(false, true)">+ toutes images inutilisées</AppButton>
        <AppButton variant="danger" :disabled="busy" @click="pruneSystem(true, true)">+ volumes ⚠️</AppButton>
      </div>
    </div>

    <!-- ── Logs modal ── -->
    <DockerLogsModal v-if="logsOpen && logsContainer" :container="logsContainer" @close="closeLogs" />
  </section>
</template>

<style scoped src="../../styles/docker-admin.css"></style>
