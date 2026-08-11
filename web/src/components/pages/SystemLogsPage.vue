<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import LogsColumn from "../organisms/LogsColumn.vue";
import ContainerLogsColumn from "../organisms/ContainerLogsColumn.vue";
import ProfileCarouselNav from "../molecules/ProfileCarouselNav.vue";
import AppSelect from "@/components/atoms/AppSelect.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import { dockerService, type DockerContainer } from "@/services/dockerService";

type ProfileKey = "sentinel" | "nexus" | "atrium" | "ops";
type ServiceProfile = { title: string; service: string };
type LogsProfile = {
  key: ProfileKey;
  label: string;
  description: string;
  services: ServiceProfile[];
};

const PROFILES: LogsProfile[] = [
  {
    key: "sentinel",
    label: "Sentinel",
    description: "Bots, workers, API et passerelle WebSocket Sentinel",
    services: [],
  },
  {
    key: "nexus",
    label: "Nexus",
    description: "Plateforme de jeux et automatisations Discord",
    services: [
      { title: "API", service: "nexus-api" },
      { title: "Bot", service: "nexus-bot" },
      { title: "Worker", service: "nexus-worker" },
    ],
  },
  {
    key: "atrium",
    label: "Atrium",
    description: "Accueil IA, bot, traitements et moteur Ollama",
    services: [
      { title: "API", service: "atrium-api" },
      { title: "Bot", service: "atrium-bot" },
      { title: "Worker", service: "atrium-worker" },
      { title: "Ollama", service: "atrium-ollama" },
    ],
  },
  {
    key: "ops",
    label: "Ops",
    description: "Exploitation, agent Docker et exposition Web",
    services: [
      { title: "API", service: "ops-api" },
      { title: "Worker", service: "ops-worker" },
      { title: "Docker", service: "docker-agent" },
      { title: "Web", service: "web" },
    ],
  },
];

const route = useRoute();
const router = useRouter();
const initialProfile = PROFILES.findIndex(profile => profile.key === route.query.profile);
const activeIndex = ref(initialProfile >= 0 ? initialProfile : 0);
const containers = ref<DockerContainer[]>([]);
const containersLoading = ref(false);
const containersError = ref("");

// Filtre de niveau global propage aux 4 colonnes. Chaque colonne garde
// la possibilite d'override en local (le select de la colonne reste).
const globalLevel = ref<"all" | "info" | "warn" | "error">("all");

const activeProfile = computed(() => PROFILES[activeIndex.value]!);

function containerFor(service: string): DockerContainer | undefined {
  return containers.value.find(container => (
    container.labels["com.docker.compose.service"] === service
  ));
}

async function loadContainers() {
  containersLoading.value = true;
  containersError.value = "";
  try {
    containers.value = await dockerService.listContainers(true);
  } catch (error) {
    containersError.value = error instanceof Error ? error.message : "Conteneurs indisponibles";
  } finally {
    containersLoading.value = false;
  }
}

function selectProfile(index: number) {
  activeIndex.value = (index + PROFILES.length) % PROFILES.length;
  void router.replace({
    query: { ...route.query, profile: activeProfile.value.key },
  });
}

watch(() => route.query.profile, (profile) => {
  const index = PROFILES.findIndex(item => item.key === profile);
  if (index >= 0) activeIndex.value = index;
});

onMounted(loadContainers);
</script>

<template>
  <AdminPageShell :title="`Logs techniques · ${activeProfile.label}`" icon="⚙️" width="wide" class="system-logs">
    <template #lede>
      Journaux techniques séparés par plateforme. Utilisez les flèches pour
      passer de Sentinel à Nexus, Atrium ou Ops sans quitter l'exploitation.
    </template>
    <template #actions>
      <div class="global-filter">
        <label for="lvl-global">Niveau global</label>
        <AppSelect id="lvl-global" v-model="globalLevel" class="lvl-select">
          <option value="all">Tous</option>
          <option value="info">Info</option>
          <option value="warn">Warn</option>
          <option value="error">Error</option>
        </AppSelect>
      </div>
    </template>

    <ProfileCarouselNav
      :title="activeProfile.label"
      :description="activeProfile.description"
      :position="activeIndex + 1"
      :total="PROFILES.length"
      @previous="selectProfile(activeIndex - 1)"
      @next="selectProfile(activeIndex + 1)"
    />

    <div v-if="activeProfile.key === 'sentinel'" class="system-grid">
      <LogsColumn title="Bots" category="bot" :force-level="globalLevel" />
      <LogsColumn title="Workers" category="worker" :force-level="globalLevel" />
      <LogsColumn title="API" category="api" :force-level="globalLevel" />
      <LogsColumn title="WebSocket" category="websocket" :force-level="globalLevel" />
    </div>

    <div v-else-if="containersLoading" class="profile-state">Chargement des services Docker…</div>
    <div v-else-if="containersError" class="profile-state error">{{ containersError }}</div>
    <div v-else class="system-grid docker-grid">
      <ContainerLogsColumn
        v-for="service in activeProfile.services"
        :key="service.service"
        :title="service.title"
        :service="service.service"
        :container="containerFor(service.service)"
        :force-level="globalLevel"
      />
    </div>
  </AdminPageShell>
</template>

<style scoped>
.system-logs { padding: 0; }
.global-filter {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
}
.lvl-select {
  height: 32px;
  padding: 4px 10px;
  font-size: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-primary);
  border-radius: var(--radius-sm);
  min-width: 140px;
}

.system-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 16px;
  align-items: start;
}
.docker-grid {
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
}
.profile-state {
  display: grid;
  min-height: 400px;
  place-items: center;
  color: var(--text-secondary);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
}
.profile-state.error { color: var(--danger); }
@media (max-width: 1400px) {
  .system-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
@media (max-width: 800px) {
  .system-grid { grid-template-columns: 1fr; }
}
</style>
