<script setup lang="ts">
// Un réglage de serveur de jeu, rendu depuis le `config_schema` du template.
//
// Partagé par la création (NexusServerCreatePage) et l'édition
// (NexusServerDetailPage) : les deux écrans lisent le MÊME schéma, ils doivent
// donc proposer le même contrôle. Tant que ce rendu était dupliqué, la page de
// détail rendait les booléens en champ texte — l'admin devait y taper
// littéralement « true » ou « false ».
//
// La valeur reste une chaîne de bout en bout : c'est ce que la base stocke
// dans `game_server_config`. La conversion est locale au contrôle.

import { computed } from "vue";
import AppToggle from "../atoms/AppToggle.vue";
import type { TemplateField } from "@/services/nexusGamesService";

const props = defineProps<{
  field: TemplateField;
  modelValue: string | undefined;
}>();

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

/// Un booléen vaut "true"/"false" en base. `TRUE` existe aussi dans les
/// schémas d'origine (EULA de Minecraft) : la comparaison ignore la casse.
const boolValue = computed(() => (props.modelValue ?? "").toLowerCase() === "true");

/// Slider uniquement quand les DEUX bornes sont connues : sans échelle, un
/// curseur n'a pas de sens et l'on retombe sur la saisie numérique.
const isSlider = computed(
  () =>
    props.field.type === "number" &&
    typeof props.field.min === "number" &&
    typeof props.field.max === "number",
);

/// Pas déduit de l'amplitude : un curseur de 0 à 100 se règle à l'unité, un
/// curseur de 300 à 86400 secondes ne peut pas se parcourir pixel par pixel.
const step = computed(() => {
  const span = (props.field.max ?? 0) - (props.field.min ?? 0);
  if (span <= 50) return 1;
  if (span <= 500) return 5;
  if (span <= 5000) return 50;
  return 100;
});

const numberValue = computed(() => {
  const parsed = Number(props.modelValue);
  return Number.isFinite(parsed) ? parsed : (props.field.min ?? 0);
});

function update(value: string | number | boolean): void {
  emit("update:modelValue", String(value));
}
</script>

<template>
  <label class="gcf" :class="`gcf-${field.type}`">
    <span class="gcf-label">{{ field.label || field.key }}</span>

    <select
      v-if="field.type === 'enum'"
      class="gcf-input"
      :value="modelValue"
      @change="update(($event.target as HTMLSelectElement).value)"
    >
      <option v-for="o in field.options ?? []" :key="o" :value="o">{{ o }}</option>
    </select>

    <!-- Interrupteur et non case à cocher : ces réglages ACTIVENT un
         comportement du serveur (PvP, vol, feu ami). Un interrupteur montre son
         état de loin, une case demande de la regarder. -->
    <AppToggle
      v-else-if="field.type === 'boolean'"
      :model-value="boolValue"
      @update:model-value="update($event)"
    />

    <!-- Curseur + valeur chiffrée : le curseur donne la position dans la plage
         autorisée d'un coup d'œil, le champ garde la saisie exacte. -->
    <span v-else-if="isSlider" class="gcf-slider">
      <input
        type="range"
        class="gcf-range"
        :min="field.min"
        :max="field.max"
        :step="step"
        :value="numberValue"
        @input="update(($event.target as HTMLInputElement).value)"
      />
      <input
        type="number"
        class="gcf-input gcf-number"
        :min="field.min"
        :max="field.max"
        :value="modelValue"
        @input="update(($event.target as HTMLInputElement).value)"
      />
    </span>

    <input
      v-else-if="field.type === 'number'"
      type="number"
      class="gcf-input"
      :min="field.min"
      :max="field.max"
      :value="modelValue"
      @input="update(($event.target as HTMLInputElement).value)"
    />

    <input
      v-else
      type="text"
      class="gcf-input"
      :maxlength="field.max_length"
      :value="modelValue"
      @input="update(($event.target as HTMLInputElement).value)"
    />

    <small v-if="field.description" class="gcf-note">{{ field.description }}</small>

    <!-- Ce que le réglage casse, pas ce qu'il fait. Séparé de la description :
         c'est précisément la ligne à ne pas rater. -->
    <small v-if="field.warning" class="gcf-warning">
      <span aria-hidden="true">⚠️</span> {{ field.warning }}
    </small>
  </label>
</template>

<style scoped>
.gcf {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.9rem;
}

/* Le libelle absorbe la hauteur libre de la cellule, ce qui colle tous les
   controles d'une meme rangee sur la meme ligne.
   Sans cela, un libelle sur trois lignes (« Regeneration au repos (joueur) »)
   poussait son curseur plus bas que ses voisins et toute la section paraissait
   decalee. */
.gcf-label {
  color: var(--text-secondary);
  flex-grow: 1;
}

.gcf-input {
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 6px 10px;
  width: 100%;
}

.gcf-input:focus {
  outline: none;
  border-color: var(--accent);
}

.gcf small {
  font-size: 0.76rem;
}

.gcf-note {
  color: var(--text-secondary);
}

.gcf-slider {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.gcf-range {
  flex: 1;
  accent-color: var(--accent);
  cursor: pointer;
}

/* La valeur chiffrée reste lisible sans manger la largeur du curseur. */
.gcf-number {
  width: 6.5rem;
  flex-shrink: 0;
}

/* Un interrupteur tient sur une ligne : le laisser occuper une colonne pleine
   gaspillait la moitié de la largeur et étirait les sections. */
.gcf.gcf-boolean {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.025);
  flex-wrap: wrap;
}

/* Description et avertissement d'un interrupteur passent sous la ligne. */
.gcf.gcf-boolean small {
  flex-basis: 100%;
}

/* Un texte libre — liste de mods, URL de modpack — se saisit mal dans une
   colonne étroite. Il prend toute la largeur de la grille. */
.gcf.gcf-text {
  grid-column: 1 / -1;
}

.gcf-warning {
  display: flex;
  align-items: flex-start;
  gap: var(--space-sm);
  margin-top: var(--space-xs);
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-sm);
  background: var(--accent-warm-bg);
  border-left: 3px solid var(--accent-warm);
  color: var(--text-primary);
  font-size: 12px;
  line-height: 1.5;
}
</style>
