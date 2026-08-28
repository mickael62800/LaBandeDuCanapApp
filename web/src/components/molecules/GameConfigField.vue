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
//
// NOMMAGE DES CLASSES. La racine porte `gcf--<type>`, avec DEUX tirets : c'est
// un modificateur, pas une classe utilitaire. Avec un seul tiret, le type
// `number` produisait `gcf-number` — le nom exact d'une classe utilitaire
// posée sur le petit champ de saisie, dont la règle `width: 6.5rem` bridait
// alors la CELLULE entière. Le curseur débordait, et description comme
// avertissement se retrouvaient compressés sur une centaine de pixels, un mot
// par ligne. Ne jamais faire porter à la racine un nom qu'une classe
// utilitaire pourrait déjà employer.
//
// Le template ne doit par ailleurs comporter QU'UNE racine : un commentaire
// place à ce niveau compte comme un nœud, le composant devient un fragment, et
// chacun de ses nœuds devient une cellule de grille indépendante.

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

/**
 * Le réglage attend-il un nombre à virgule ?
 *
 * La plupart des taux Palworld sont des décimaux (gain d'expérience de 0,1 à
 * 20, vitesse du jour, dégâts…). Avec un pas entier, le navigateur refusait
 * « 1,5 » et le curseur sautait de 1 à 2 : la moitié des réglages du jeu
 * étaient inatteignables. On le déduit des bornes et du défaut, qui portent
 * déjà l'information.
 */
const isDecimal = computed(() =>
  [props.field.min, props.field.max, Number(props.field.default)].some(
    (v) => typeof v === "number" && Number.isFinite(v) && !Number.isInteger(v),
  ),
);

/// Pas déduit de l'amplitude : un curseur de 0 à 100 se règle à l'unité, un
/// curseur de 300 à 86400 secondes ne peut pas se parcourir pixel par pixel.
const step = computed(() => {
  if (isDecimal.value) return 0.1;
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
  <label class="gcf" :class="`gcf--${field.type}`">
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
        class="gcf-input"
        :min="field.min"
        :max="field.max"
        :step="step"
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
      :step="step"
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
/* Chaque reglage est une CARTE. Auparavant seuls les interrupteurs en avaient
   une : a l'ecran, une liste deroulante ou un curseur flottait sans limite
   visible a cote d'un booleen encadre, et rien ne disait ou finissait un
   reglage et ou commencait le suivant. */
.gcf {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: var(--space-sm) var(--space-md);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.025);
  font-size: 0.9rem;
  /* Sans cela, une cellule ne peut pas descendre sous la largeur intrinseque
     de son contenu : un curseur ou une longue option deborderait la colonne. */
  min-width: 0;
}

/* Deux lignes reservees : la plupart des libelles y tiennent, donc les
   controles d'une meme rangee s'alignent sans rien etirer.
   L'ancienne version employait `flex-grow: 1` pour absorber la hauteur libre
   de la cellule. Cela marchait tant que les cellules restaient basses, mais
   des qu'un reglage voisin portait un avertissement long, la rangee devenait
   haute de plusieurs centaines de pixels et le libelle avalait tout l'ecart :
   le controle partait au fond de la cellule, separe de son libelle. La grille
   pose desormais `align-items: start` et le libelle ne s'etire plus. */
.gcf-label {
  color: var(--text-secondary);
  font-size: 0.95rem;
  font-weight: 600;
  min-height: 2.2em;
  line-height: 1.3;
}

/* Un mot plus long que la colonne (une URL de modpack) ne doit pas elargir la
   cellule ni deborder : il se coupe. */
.gcf-note,
.gcf-warning {
  overflow-wrap: anywhere;
}

.gcf-input {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 8px 12px;
  font-size: 0.98rem;
  width: 100%;
}

.gcf-input:focus {
  outline: none;
  border-color: var(--accent);
}

.gcf small {
  font-size: 0.88rem;
  line-height: 1.45;
}

.gcf-note {
  color: var(--text-secondary);
  margin-top: 2px;
}

.gcf-slider {
  display: flex;
  align-items: center;
  gap: var(--space-md);
}

.gcf-range {
  flex: 1;
  accent-color: var(--accent);
  cursor: pointer;
}

.gcf-slider .gcf-input {
  width: 7rem;
  flex-shrink: 0;
}

/* Un interrupteur tient sur une ligne : le laisser occuper une colonne pleine
   gaspillait la moitié de la largeur et étirait les sections. La carte, elle,
   est commune a tous les reglages (voir `.gcf`). */
.gcf.gcf--boolean {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  flex-wrap: wrap;
}

/* Description et avertissement d'un interrupteur passent sous la ligne. */
.gcf.gcf--boolean small {
  flex-basis: 100%;
}

/* Cote a cote avec l'interrupteur, le libelle n'a pas de controle a aligner
   sous lui : les deux lignes reservees creuseraient la carte pour rien. */
.gcf.gcf--boolean .gcf-label {
  min-height: 0;
}

/* Un texte libre — liste de mods, URL de modpack — se saisit mal dans une
   colonne étroite. Il prend toute la largeur de la grille. */
.gcf.gcf--text {
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
  font-size: 0.88rem;
  line-height: 1.5;
}
</style>
