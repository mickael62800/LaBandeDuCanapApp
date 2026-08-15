<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute } from "vue-router";
import { storeToRefs } from "pinia";
import type { BotDefinition } from "../../types";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useBotDefinitions } from "../../composables/useBotDefinitions";
import { useBotEnabledStatus } from "../../composables/useBotEnabledStatus";
import { useBotEnabledStatusStore } from "@/stores/botEnabledStatusStore";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import ComponentSelectorSection from "../organisms/ComponentSelectorSection.vue";
import ComponentConfigForm from "../organisms/ComponentConfigForm.vue";
import AutomodAnalysisHistory from "../organisms/AutomodAnalysisHistory.vue";

const route = useRoute();
const { selectedGuildId, selectedGuild } = useGuildSelector();
const { fetchConfigs } = useBotEnabledStatus();

// Une seule source de verite : le store. La page ne fait pas de
// fetch separe. Le store est deja charge par useAppInit + le watch
// dans useBotEnabledStatus a la selection de guild.
const botEnabledStore = useBotEnabledStatusStore();
const { configs } = storeToRefs(botEnabledStore);

const definitions = ref<BotDefinition[]>([]);
const selectedComponent = ref<string | null>(null);

function isWorker(botName: string): boolean {
  return botName.endsWith("-worker");
}

// NB : welcome-bot dispose d'une page de config dediee (/welcome, UX riche),
// mais il DOIT rester liste ici — c'est le seul endroit ou basculer son
// interrupteur `enabled`. L'exclure le rendait impossible a activer (et la
// tuile « Bienvenue », gardee par requiredBot, n'apparaissait donc jamais).
const moduleDefinitions = computed(() =>
  definitions.value.filter((d) => !isWorker(d.bot_name)),
);
const workerDefinitions = computed(() =>
  definitions.value.filter((d) => isWorker(d.bot_name)),
);

const selectedDefinition = computed(() =>
  definitions.value.find((d) => d.bot_name === selectedComponent.value) ?? null,
);

async function fetchDefinitions() {
  try {
    const { ensure } = useBotDefinitions();
    definitions.value = await ensure();
  } catch (e) {
    console.error("Erreur chargement definitions:", e);
  }
}

function selectComponent(name: string) {
  selectedComponent.value = name;
}

// fetchConfigs() invalide + recharge le store (la seule source).
// Appele apres un save dans le formulaire.
async function reloadAfterSave() {
  await fetchConfigs();
}

onMounted(async () => {
  await fetchDefinitions();
  // Lien direct depuis une tuile du tableau de bord : ?bot=<bot_name>
  // présélectionne le module concerné (ex. /component-config?bot=nasa-apod-bot).
  const wanted = route.query.bot;
  if (typeof wanted === "string" && definitions.value.some((d) => d.bot_name === wanted)) {
    selectComponent(wanted);
  } else if (moduleDefinitions.value.length > 0) {
    selectComponent(moduleDefinitions.value[0].bot_name);
  }
});
</script>

<template>
  <AdminPageShell title="Configuration des composants">
    <template #lede>
      Parametrer chaque composant pour le serveur selectionne
    </template>

    <div v-if="!selectedGuildId" class="empty-state">
      <p>Selectionnez un serveur dans la barre laterale pour configurer les composants.</p>
    </div>

    <template v-else>
      <div class="server-info">
        <span class="server-label">Serveur :</span>
        <span class="server-name">{{ selectedGuild?.name }}</span>
      </div>

      <ComponentSelectorSection
        title="Modules"
        :definitions="moduleDefinitions"
        :selected-key="selectedComponent"
        @select="selectComponent"
      />

      <ComponentSelectorSection
        v-if="workerDefinitions.length > 0"
        title="Workers"
        :definitions="workerDefinitions"
        :selected-key="selectedComponent"
        @select="selectComponent"
      />

      <ComponentConfigForm
        v-if="selectedDefinition"
        :definition="selectedDefinition"
        :configs="configs"
        :guild-id="selectedGuildId"
        @saved="reloadAfterSave"
      />

      <div v-else class="select-hint">
        <strong>Choisissez un composant</strong>
        <span>Ses réglages apparaîtront ici pour {{ selectedGuild?.name }}.</span>
      </div>

      <!-- Vue debug temporaire : historique des analyses automod. -->
      <AutomodAnalysisHistory
        v-if="selectedComponent === 'automod-bot' && selectedGuildId"
        :guild-id="selectedGuildId"
      />
    </template>
  </AdminPageShell>
</template>

<style scoped>
.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: var(--text-secondary);
  font-size: 15px;
}

.server-info {
  margin-bottom: 20px;
  padding: 10px 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  font-size: 14px;
}
.server-label { color: var(--text-secondary); margin-right: 8px; }
.server-name { font-weight: 600; color: var(--text-primary); }
.select-hint {
  display: grid;
  gap: 4px;
  margin-top: 24px;
  padding: 28px;
  text-align: center;
  color: var(--text-secondary);
  border: 1px dashed var(--border);
  border-radius: var(--radius-md);
}
.select-hint strong { color: var(--text-primary); }
</style>
