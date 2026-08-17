<script setup lang="ts">
import { computed, ref } from "vue";
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
 * Jaquettes candidates selon l'état du serveur, de la plus précise à la moins :
 * - En ligne (Running) : jaquette normale "LE SERVEUR EST OUVERT !" (ex: palworld_game.jpg)
 * - En attente (Scheduled / Starting) : jaquette "LE SERVEUR OUVRE BIENTÔT !" (ex: palworld_game_attente.jpg)
 * - Hors ligne (Stopped / Created / Error) : jaquette "LE SERVEUR EST FERMÉ !" (ex: palworld_game_offline.jpg)
 *
 * Les variantes existent tantôt en .jpg, tantôt en .png : on tente les deux.
 * On ne retombe jamais sur la jaquette "ouvert" pour un serveur fermé, sinon la
 * carte annonce le contraire de l'état réel ; à défaut, l'icône prend le relais.
 * `nexus-bot/src/game_portal.rs` applique la même règle côté Discord.
 */
function coverCandidates(server: PublicGameServer): string[] {
  const url = server.cover_image_url;
  if (!url) return [];

  // L'état vient de l'API, qui l'a calculé à partir de la fenêtre horaire ET
  // du conteneur. Le site ne rejoue pas cette règle : quand chacun avait la
  // sienne, Discord et le site racontaient la même session différemment.
  // Repli sur le statut brut si l'API ne le renseigne pas encore.
  const state =
    server.display_state
    ?? (server.status === "running"
      ? "open"
      : server.status === "scheduled" || server.status === "starting"
        ? "waiting"
        : "closed");

  if (state === "open") {
    return [url];
  }

  const dot = url.lastIndexOf(".");
  if (dot === -1) return [];

  const base = url.substring(0, dot).replace(/_(offline|waiting|attente)$/i, "");
  const ext = url.substring(dot).toLowerCase();

  const suffix = state === "waiting" ? "_attente" : "_offline";
  const alt = ext === ".png" ? ".jpg" : ".png";
  return [`${base}${suffix}${ext}`, `${base}${suffix}${alt}`];
}

/** Index de la candidate en cours par serveur, avancé à chaque échec de chargement. */
const coverStep = ref<Record<string, number>>({});

function coverImageFor(server: PublicGameServer): string | null {
  const candidates = coverCandidates(server);
  return candidates[coverStep.value[server.id] ?? 0] ?? null;
}

function onCoverError(server: PublicGameServer): void {
  coverStep.value[server.id] = (coverStep.value[server.id] ?? 0) + 1;
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
          v-if="coverImageFor(server)"
          :src="coverImageFor(server)!"
          :alt="server.game"
          @error="onCoverError(server)"
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
