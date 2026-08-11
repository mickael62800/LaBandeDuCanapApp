<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@/api/events-api";
import { getApiBaseUrl } from "../../utils/api";

const apiStatus = ref<"ok" | "down" | "checking">("checking");
let unlisten: UnlistenFn | null = null;
let fallbackInterval: ReturnType<typeof setInterval> | null = null;

async function checkApi() {
  try {
    const baseUrl = await getApiBaseUrl();
    const resp = await fetch(`${baseUrl}/health`, { signal: AbortSignal.timeout(3000) });
    apiStatus.value = resp.ok ? "ok" : "down";
  } catch {
    apiStatus.value = "down";
  }
}

onMounted(async () => {
  // Check initial au demarrage
  await checkApi();

  // Ecouter les heartbeats via WebSocket — chaque heartbeat confirme que l'API est up
  unlisten = await listen<{ event: string }>("ws:event", (e) => {
    if (e.payload.event === "bot_heartbeat") {
      apiStatus.value = "ok";
    }
  });

  // Fallback : si aucun heartbeat recu en 90s, verifier via HTTP
  fallbackInterval = setInterval(async () => {
    await checkApi();
  }, 90000);
});

onUnmounted(() => {
  if (unlisten) unlisten();
  if (fallbackInterval) clearInterval(fallbackInterval);
});
</script>

<template>
  <div v-if="apiStatus === 'down'" class="connection-banner">
    <span class="banner-icon">!</span>
    <span class="banner-text">Connexion au serveur perdue. Certaines donnees peuvent etre indisponibles.</span>
    <button class="banner-retry" @click="checkApi">Verifier</button>
  </div>
</template>

<style scoped>
.connection-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 20px;
  background: linear-gradient(90deg, rgba(239, 68, 68, 0.15), rgba(239, 68, 68, 0.05));
  border-bottom: 1px solid rgba(239, 68, 68, 0.3);
  font-size: 13px;
  color: #fca5a5;
}

.banner-icon {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--danger);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}

.banner-text {
  flex: 1;
}

.banner-retry {
  background: transparent;
  border: 1px solid rgba(239, 68, 68, 0.5);
  color: #fca5a5;
  border-radius: var(--radius-sm);
  padding: 4px 12px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}

.banner-retry:hover {
  background: rgba(239, 68, 68, 0.2);
}
</style>
