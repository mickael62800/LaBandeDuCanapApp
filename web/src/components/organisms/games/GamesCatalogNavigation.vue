<script setup lang="ts">
import { computed } from "vue";
import { badgeCanaux, type GameCard } from "@/games/catalog";

const props = defineProps<{
  games: GameCard[];
  activeKey: string;
}>();

const emit = defineEmits<{
  select: [key: string];
  shift: [step: number];
}>();

const activeGame = computed(() =>
  props.games.find((game) => game.key === props.activeKey) ?? props.games[0],
);
</script>

<template>
  <section class="jx-block" aria-label="Choix du jeu">
    <div class="jx-carrousel">
      <button
        v-if="games.length > 1"
        type="button"
        class="jx-fleche-nav"
        aria-label="Jeu précédent"
        @click="emit('shift', -1)"
      >‹</button>

      <ul class="jx-vignettes">
        <li v-for="game in games" :key="game.key">
          <button
            type="button"
            class="jx-vignette"
            :class="{ active: game.key === activeKey }"
            :style="{ '--c': game.couleur }"
            :aria-current="game.key === activeKey ? 'true' : undefined"
            @click="emit('select', game.key)"
          >
            <span class="jx-vignette-emoji" aria-hidden="true">{{ game.emoji }}</span>
            <span class="jx-vignette-nom">{{ game.nom }}</span>
            <span class="jx-vignette-tag" :class="{ double: game.canaux.length > 1 }">
              {{ badgeCanaux(game) }}
            </span>
          </button>
        </li>
      </ul>

      <button
        v-if="games.length > 1"
        type="button"
        class="jx-fleche-nav"
        aria-label="Jeu suivant"
        @click="emit('shift', 1)"
      >›</button>
    </div>

    <p v-if="activeGame" class="jx-pitch">{{ activeGame.pitch }}</p>
  </section>
</template>
