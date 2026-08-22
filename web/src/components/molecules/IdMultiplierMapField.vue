<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { errMsg } from "@/utils/errMsg";
import { guildChannelsService } from "@/services/guildChannelsService";
import { discordRolesService } from "@/services/discordRolesService";
import type { DiscordChannelInfo, DiscordRole } from "@/types";

type Kind = "channel" | "channel-all" | "role";

function channelLabel(c: DiscordChannelInfo): string {
  switch (c.kind) {
    case "voice":
      return `🔊 ${c.name}`;
    case "stage":
      return `🎙️ ${c.name}`;
    case "announcement":
      return `📢 ${c.name}`;
    default:
      return `# ${c.name}`;
  }
}

const props = defineProps<{
  modelValue: string;
  guildId: string | null;
  kind: Kind;
  valueLabel?: string;
  valueStep?: number;
  valueMin?: number;
  valueMax?: number;
  valueDefault?: number;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

interface Entry {
  id: string;
  value: number;
}

interface Option {
  id: string;
  label: string;
  color?: string;
}

const channels = ref<DiscordChannelInfo[]>([]);
const roles = ref<DiscordRole[]>([]);
const loading = ref(false);
const errorMsg = ref("");
const pickedId = ref("");
const pickedValue = ref<number>(props.valueDefault ?? 1);

async function load() {
  if (!props.guildId) {
    channels.value = [];
    roles.value = [];
    return;
  }
  loading.value = true;
  errorMsg.value = "";
  try {
    // `?? []` : ces refs sont ensuite parcourues par `options`, qui suppose un
    // tableau. Une reponse vide ou malformee de l'API y devenait `undefined`
    // et faisait echouer le rendu du champ entier — l'ecran perdait le
    // selecteur sans rien dire, la ou une liste vide se voit et se comprend.
    if (props.kind === "channel") {
      channels.value = (await guildChannelsService.listTextChannels(props.guildId)) ?? [];
    } else if (props.kind === "channel-all") {
      channels.value = (await guildChannelsService.listAllChannels(props.guildId)) ?? [];
    } else {
      roles.value = (await discordRolesService.getAll(props.guildId)) ?? [];
    }
  } catch (e) {
    errorMsg.value = errMsg(e);
  } finally {
    loading.value = false;
  }
}

watch(() => props.guildId, load);
watch(() => props.kind, load);
onMounted(load);

const options = computed<Option[]>(() => {
  if (props.kind === "channel" || props.kind === "channel-all") {
    return channels.value.map((c) => ({ id: c.id, label: channelLabel(c) }));
  }
  const sorted = [...roles.value].sort((a, b) => (b.position ?? 0) - (a.position ?? 0));
  return sorted.map((r) => ({
    id: r.id,
    label: `@${r.name}`,
    color: r.color ? "#" + r.color.toString(16).padStart(6, "0") : undefined,
  }));
});

const entries = computed<Entry[]>(() => {
  return props.modelValue
    .split(/[\n,;]+/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const [idPart, valPart] = line.split(":");
      const id = (idPart ?? "").trim();
      const value = Number((valPart ?? "").trim());
      if (!id || !Number.isFinite(value)) return null;
      return { id, value };
    })
    .filter((e): e is Entry => e !== null);
});

const usedIds = computed(() => new Set(entries.value.map((e) => e.id)));

const availableOptions = computed(() =>
  options.value.filter((o) => !usedIds.value.has(o.id)),
);

function labelFor(id: string): string {
  const opt = options.value.find((o) => o.id === id);
  return opt?.label ?? `ID ${id}`;
}

function colorFor(id: string): string | undefined {
  return options.value.find((o) => o.id === id)?.color;
}

function serialize(list: Entry[]): string {
  return list.map((e) => `${e.id}:${e.value}`).join("\n");
}

function add() {
  if (!pickedId.value) return;
  if (!Number.isFinite(pickedValue.value)) return;
  if (usedIds.value.has(pickedId.value)) return;
  const next = [...entries.value, { id: pickedId.value, value: pickedValue.value }];
  emit("update:modelValue", serialize(next));
  pickedId.value = "";
  pickedValue.value = props.valueDefault ?? 1;
}

function remove(id: string) {
  emit("update:modelValue", serialize(entries.value.filter((e) => e.id !== id)));
}

function updateValue(id: string, value: number) {
  if (!Number.isFinite(value)) return;
  emit(
    "update:modelValue",
    serialize(entries.value.map((e) => (e.id === id ? { ...e, value } : e))),
  );
}

const valueLabel = computed(() => props.valueLabel ?? "Multiplicateur");
const step = computed(() => props.valueStep ?? 0.25);
const placeholderTxt = computed(() =>
  props.kind === "role" ? "— Choisir un rôle —" : "— Choisir un salon —",
);
</script>

<template>
  <div class="map-field">
    <!-- Picker row -->
    <div class="picker-row">
      <select
        v-model="pickedId"
        class="picker-select"
        :disabled="loading || !guildId || availableOptions.length === 0"
      >
        <option value="">
          {{
            loading
              ? "Chargement..."
              : availableOptions.length === 0
                ? "— Aucun élément disponible —"
                : placeholderTxt
          }}
        </option>
        <option
          v-for="o in availableOptions"
          :key="o.id"
          :value="o.id"
          :style="o.color ? { color: o.color } : undefined"
        >
          {{ o.label }}
        </option>
      </select>

      <div class="value-input-wrap">
        <input
          v-model.number="pickedValue"
          type="number"
          :step="step"
          :min="valueMin"
          :max="valueMax"
          class="value-input"
          :placeholder="String(valueDefault ?? 1)"
        />
        <span class="value-suffix">×</span>
      </div>

      <button
        type="button"
        class="btn-add"
        :disabled="!pickedId || !Number.isFinite(pickedValue)"
        @click="add"
      >
        + Ajouter
      </button>
    </div>

    <span v-if="errorMsg" class="err">{{ errorMsg }}</span>

    <!-- Liste des entrees -->
    <div v-if="entries.length > 0" class="entries">
      <div v-for="e in entries" :key="e.id" class="entry">
        <span
          class="entry-label"
          :style="colorFor(e.id) ? { color: colorFor(e.id) } : undefined"
          :title="labelFor(e.id)"
        >
          {{ labelFor(e.id) }}
        </span>
        <div class="entry-value-wrap">
          <input
            type="number"
            :value="e.value"
            :step="step"
            :min="valueMin"
            :max="valueMax"
            class="entry-value"
            @change="updateValue(e.id, Number(($event.target as HTMLInputElement).value))"
          />
          <span class="value-suffix">×</span>
        </div>
        <button
          type="button"
          class="btn-remove"
          :title="`Retirer ${labelFor(e.id)}`"
          @click="remove(e.id)"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
            <path
              d="M3 3l8 8M11 3l-8 8"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </div>
    </div>

    <p v-else class="empty">
      Aucun {{ kind === "role" ? "rôle" : "salon" }} configuré — utilise le sélecteur ci-dessus
      pour ajouter un {{ valueLabel.toLowerCase() }}.
    </p>
  </div>
</template>

<style scoped>
.map-field {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
}

.picker-row {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 6px;
  align-items: stretch;
}

.picker-select {
  padding: 8px 28px 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm, 6px);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg width='10' height='6' viewBox='0 0 10 6' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%239ca3af' stroke-width='1.5' fill='none' stroke-linecap='round'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
  min-width: 0;
}

.picker-select:focus {
  outline: none;
  border-color: var(--accent);
}

.picker-select:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.value-input-wrap {
  display: inline-flex;
  align-items: center;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm, 6px);
  background: var(--bg-card);
  padding: 0 10px 0 8px;
  width: 90px;
}

.value-input-wrap:focus-within {
  border-color: var(--accent);
}

.value-input {
  width: 100%;
  padding: 7px 0;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 13px;
  font-variant-numeric: tabular-nums;
  text-align: right;
  outline: none;
  -moz-appearance: textfield;
}

.value-input::-webkit-outer-spin-button,
.value-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  appearance: none;
  margin: 0;
}

.value-suffix {
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  margin-left: 4px;
}

.btn-add {
  padding: 0 14px;
  border: none;
  border-radius: var(--radius-sm, 6px);
  background: var(--accent);
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.12s;
}

.btn-add:hover:not(:disabled) {
  background: var(--accent-hover, var(--accent-hover));
}

.btn-add:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.err {
  font-size: 11px;
  color: var(--danger, var(--danger));
}

.entries {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.entry {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 8px;
  align-items: center;
  padding: 6px 8px 6px 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm, 6px);
}

.entry-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.entry-value-wrap {
  display: inline-flex;
  align-items: center;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm, 6px);
  background: var(--bg-secondary, var(--bg-card));
  padding: 0 8px 0 6px;
  width: 80px;
}

.entry-value-wrap:focus-within {
  border-color: var(--accent);
}

.entry-value {
  width: 100%;
  padding: 5px 0;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  text-align: right;
  outline: none;
  -moz-appearance: textfield;
}

.entry-value::-webkit-outer-spin-button,
.entry-value::-webkit-inner-spin-button {
  -webkit-appearance: none;
  appearance: none;
  margin: 0;
}

.btn-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm, 6px);
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.12s, color 0.12s, border-color 0.12s;
}

.btn-remove:hover {
  background: var(--danger-bg, rgba(237, 66, 69, 0.15));
  border-color: var(--danger, var(--danger));
  color: var(--danger, var(--danger));
}

.empty {
  margin: 0;
  font-size: 12px;
  color: var(--text-secondary);
  font-style: italic;
}

@media (max-width: 600px) {
  .picker-row {
    grid-template-columns: 1fr;
  }
  .value-input-wrap {
    width: 100%;
  }
}
</style>
