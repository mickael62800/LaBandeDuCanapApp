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

/**
 * Calcule l'image spécifique selon l'état du serveur :
 * - Online (Running) : suffixe `_online` (ou image de base)
 * - Attente (Scheduled / Starting) : suffixe `_waiting` ou `_attente`
 * - Offline (Stopped / Created / Error) : suffixe `_offline`
 */
function coverImageFor(server: PublicGameServer): string | null {
  if (!server.cover_image_url) return null;
  const url = server.cover_image_url;
  const dot = url.lastIndexOf(".");
  if (dot === -1) return url;

  const base = url.substring(0, dot);
  const ext = url.substring(dot);

  // Élimine tout suffixe d'état déjà présent dans l'URL de base s'il y en a un
  const cleanBase = base.replace(/_(online|offline|waiting|attente)$/i, "");

  const status = server.status || (server.online ? "running" : "stopped");

  if (status === "running") {
    return `${cleanBase}_online${ext}`;
  } else if (status === "scheduled" || status === "starting") {
    return `${cleanBase}_waiting${ext}`;
  } else {
    return `${cleanBase}_offline${ext}`;
  }
}

/** Libellé d'état lisible */
function stateLabel(server: PublicGameServer): string {
  const status = server.status || (server.online ? "running" : "stopped");
  switch (status) {
    case "running":
      return server.game;
    case "scheduled":
      return "En attente d'ouverture";
    case "starting":
      return "Démarrage en cours…";
    case "stopping":
      return "Arrêt en cours…";
    case "error":
      return "En erreur";
    default:
      return "Hors ligne";
  }
}
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
      <li v-for="server in sortedServers" :key="server.id" class="mb-game" :class="{ off: !server.online, waiting: server.status === 'scheduled' || server.status === 'starting' }">
        <span v-if="server.online && server.player_count" class="mb-badge">{{ server.player_count }} EN JEU</span>
        <img
          v-if="server.cover_image_url"
          :src="coverImageFor(server)!"
          :alt="server.game"
          @error="($event.target as HTMLImageElement).src = server.cover_image_url!"
        />
        <div v-else class="mb-game-fallback" aria-hidden="true">{{ server.icon || "🎮" }}</div>
        <div class="mb-game-in">
          <strong>{{ server.name }}</strong>
          <span class="mb-game-state">
            <span class="mb-pip" :class="server.online ? 'on' : server.status === 'scheduled' || server.status === 'starting' ? 'waiting' : 'off'"></span>
            {{ stateLabel(server) }}
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
