<script setup lang="ts">
import { computed } from "vue";
import AppButton from "@/components/atoms/AppButton.vue";
import type { PublicGameServer } from "@/services/publicGamesService";

const props = defineProps<{
  servers: PublicGameServer[];
  loading: boolean;
  canReveal?: boolean;
  busyRevealId?: string | null;
}>();
defineEmits<{ reveal: [server: PublicGameServer] }>();
const sortedServers = computed(() =>
  [...props.servers].sort((a, b) => Number(b.online) - Number(a.online)),
);
const playersOnline = computed(() =>
  props.servers.reduce((total, server) => total + (server.online ? server.player_count : 0), 0),
);
</script>

<template>
  <section class="mb-block">
    <h2>
      Nos serveurs de jeu
      <span v-if="playersOnline" class="mb-count">{{ playersOnline }} joueur(s) en ligne</span>
    </h2>
    <p v-if="loading" class="mb-hint">Chargement des serveurs…</p>
    <p v-else-if="!servers.length" class="mb-vide">
      Aucun serveur de jeu déclaré. Ils apparaîtront ici avec leur jaquette et le nombre de joueurs connectés.
    </p>
    <ul v-else class="mb-games">
      <li v-for="server in sortedServers" :key="server.id" class="mb-game" :class="{ off: !server.online }">
        <span v-if="server.online && server.player_count" class="mb-badge">{{ server.player_count }} EN JEU</span>
        <img v-if="server.cover_image_url" :src="server.cover_image_url" :alt="server.game" />
        <div v-else class="mb-game-fallback" aria-hidden="true">{{ server.icon || "🎮" }}</div>
        <div class="mb-game-in">
          <strong>{{ server.name }}</strong>
          <span class="mb-game-state">
            <span class="mb-pip" :class="server.online ? 'on' : 'off'"></span>
            {{ server.online ? server.game : "Hors ligne" }}
          </span>
          <span v-if="server.online && server.address" class="mb-game-addr">{{ server.address }}</span>
          <span v-else-if="server.online && server.port" class="mb-game-addr">Port {{ server.port }}</span>
          <span v-else-if="server.online" class="mb-game-addr muted">Adresse bientôt révélée</span>
          <AppButton
            v-if="canReveal && server.online && !server.address_revealed"
            class="mb-reveal"
            variant="warning"
            size="xs"
            :disabled="busyRevealId === server.id"
            @click="$emit('reveal', server)"
          >
            {{ busyRevealId === server.id ? "Révélation…" : "Révéler maintenant" }}
          </AppButton>
        </div>
      </li>
    </ul>
  </section>
</template>
