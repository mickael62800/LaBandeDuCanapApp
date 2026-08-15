<script setup lang="ts">
import { computed } from "vue";
import type { BotDefinition } from "../../types";
import { useBotEnabledStatus } from "@/composables/useBotEnabledStatus";

const props = defineProps<{
  title: string;
  definitions: BotDefinition[];
  selectedKey: string | null;
}>();

const emit = defineEmits<{
  (e: "select", name: string): void;
}>();

// On utilise enabledMap (ref reactif) directement plutot que la
// fonction isBotEnabled, sinon Vue ne re-render pas le badge OFF
// apres invalidation du store (la closure casse le tracking).
const { enabledMap } = useBotEnabledStatus();

// Fail-closed, comme le bot : sans ligne `enabled` explicite, le module est
// inactif et la carte s'affiche en rouge avec le badge OFF.
function isOn(botName: string): boolean {
  return enabledMap.value[botName] === true;
}

function displayedParamCount(def: BotDefinition): number {
  const schema = Array.isArray(def.config_schema) ? def.config_schema : [];
  if (def.bot_name === "welcome-bot") return 1;
  if (def.bot_name === "automod-bot") return schema.filter((f) => !f.key.startsWith("score_")).length;
  return schema.length;
}

const enabledCount = computed(() => props.definitions.filter((definition) => isOn(definition.bot_name)).length);
</script>

<template>
  <section class="component-section">
    <div class="section-header">
      <h2 class="section-heading">{{ title }}</h2>
      <span class="section-count">{{ enabledCount }}/{{ definitions.length }} actifs</span>
    </div>
    <div class="component-grid">
      <button
        v-for="def in definitions"
        :key="def.bot_name"
        type="button"
        class="component-card"
        :class="{
          active: selectedKey === def.bot_name,
          'is-disabled': !isOn(def.bot_name),
        }"
        :aria-pressed="selectedKey === def.bot_name"
        @click="emit('select', def.bot_name)"
      >
        <div class="component-card-header">
          <div class="component-name">{{ def.display_name }}</div>
          <span v-if="!isOn(def.bot_name)" class="off-pill" title="Désactivé pour cette guild">OFF</span>
        </div>
        <div class="component-desc">{{ def.description }}</div>
        <div class="component-params">
          {{ displayedParamCount(def) }} parametre{{ displayedParamCount(def) > 1 ? "s" : "" }}
        </div>
      </button>
    </div>
  </section>
</template>

<style scoped>
.component-section { margin-bottom: 24px; }

.section-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border);
}

.section-heading {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0;
}

.section-count {
  font-size: 11px;
  font-weight: 600;
  color: var(--accent);
  background: rgba(99, 102, 241, 0.12);
  padding: 2px 8px;
  border-radius: var(--radius-md);
}

.component-grid {
  display: grid;
  /* 1 col mobile, 2 tablette, jusqu'a 3 desktop (4 sur tres grand ecran) :
     les descriptions ont la place de respirer au lieu d'un mur de cartes. */
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 12px;
}
@media (min-width: 1900px) {
  .component-grid { grid-template-columns: repeat(4, 1fr); }
}

.component-card {
  width: 100%;
  text-align: left;
  font: inherit;
  color: inherit;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 16px;
  cursor: pointer;
  transition: border-color var(--transition-fast), transform var(--transition-fast), background var(--transition-fast);
}

.component-card:hover { border-color: var(--accent); transform: translateY(-1px); }
.component-card:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.component-card.active {
  border-color: var(--accent);
  background: rgba(99, 102, 241, 0.08);
}

/* Composant desactive pour cette guild : bordure rouge + tint subtil. */
.component-card.is-disabled {
  border-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 6%, var(--bg-secondary));
}
.component-card.is-disabled:hover {
  border-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 10%, var(--bg-secondary));
}
.component-card.is-disabled.active {
  border-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 14%, var(--bg-secondary));
}
.off-pill {
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.6px;
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  background: var(--danger);
  color: white;
}

.component-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.component-name {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-primary);
}

.component-desc {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.component-params {
  font-size: 11px;
  color: var(--accent);
  font-weight: 500;
}

@media (max-width: 640px) {
  .component-grid {
    grid-template-columns: 1fr;
    gap: 10px;
  }
  .component-card { padding: 12px; }
}
</style>
