<script setup lang="ts">
import type { Rank } from "@/services/gamesService";

defineProps<{ ranking: Rank[] }>();
const fmtCoins = (value: number) => value.toLocaleString("fr-FR");
</script>

<template>
  <section class="jx-block">
    <h2>Les plus riches</h2>
    <p v-if="!ranking.length" class="jx-vide">Personne n'a encore de coins.</p>
    <ol v-else class="jx-rangs">
      <li v-for="rank in ranking" :key="rank.rank" class="jx-rang" :class="{ moi: rank.is_me }">
        <span class="jx-place">{{ rank.rank }}</span>
        <span class="jx-nom">{{ rank.username || "Un membre" }}</span>
        <span class="jx-coins">{{ fmtCoins(rank.coins) }}</span>
      </li>
    </ol>
  </section>
</template>
