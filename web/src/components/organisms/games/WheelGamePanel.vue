<script setup lang="ts">
import ActionButton from "@/components/atoms/ActionButton.vue";
import type { SpinResult } from "@/services/gamesService";

export interface WheelCaseView {
  key: string;
  emoji: string;
}

defineProps<{
  cases: WheelCaseView[];
  sector: number;
  background: string;
  angle: number;
  spinning: boolean;
  alreadyPlayed: boolean;
  error: string | null;
  result: SpinResult | null;
}>();

const emit = defineEmits<{ spin: [] }>();
const fmtCoins = (value: number) => value.toLocaleString("fr-FR");
</script>

<template>
  <section class="jx-block">
    <h2>La Roue du Destin <span class="jx-count">un tirage par jour</span></h2>

    <div class="jx-roue-zone">
      <div class="jx-roue-wrap">
        <span class="jx-fleche" aria-hidden="true"></span>
        <div
          class="jx-roue"
          :style="{ background, transform: `rotate(${angle}deg)` }"
          aria-label="Roue du Destin"
        >
          <span
            v-for="(wheelCase, index) in cases"
            :key="wheelCase.key"
            class="jx-case"
            :style="{ transform: `rotate(${index * sector + sector / 2}deg)` }"
          >
            <span class="jx-case-in">{{ wheelCase.emoji }}</span>
          </span>
        </div>
      </div>

      <div class="jx-roue-cote">
        <ActionButton
          size="lg"
          :disabled="spinning || alreadyPlayed"
          @click="emit('spin')"
        >
          <template v-if="spinning">Ça tourne…</template>
          <template v-else-if="alreadyPlayed">Reviens demain</template>
          <template v-else>Tirer la Roue</template>
        </ActionButton>

        <p v-if="error" :class="alreadyPlayed ? 'jx-vide' : 'jx-alerte'">{{ error }}</p>

        <div v-else-if="result" class="jx-resultat" :class="{ rare: result.is_memorable }">
          <strong>{{ result.case_label }}</strong>
          <span
            v-if="result.payout !== 0"
            class="jx-gain"
            :class="result.payout > 0 ? 'plus' : 'moins'"
          >
            {{ result.payout > 0 ? "+" : "" }}{{ fmtCoins(result.payout) }} coins
          </span>
          <span v-else class="jx-gain neutre">Rien. Du tout.</span>
          <span class="jx-apres">Nouveau solde : {{ fmtCoins(result.balance_after) }}</span>
        </div>

        <p v-else class="jx-vide">
          Dix cases, de la ruine à la licorne. Le tirage est le même que
          celui de <code>/roue</code> sur Discord.
        </p>
      </div>
    </div>
  </section>
</template>
