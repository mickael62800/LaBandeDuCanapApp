<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import StatusDot from "../atoms/StatusDot.vue";
import NotificationModal from "./NotificationModal.vue";
import { useAuth } from "../../composables/useAuth";
import { useNotifications } from "../../composables/useNotifications";
import { useRealtime } from "../../composables/useRealtime";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { siteConfig } from "@/siteConfig";
import { useSidebar } from "../../composables/useSidebar";
import { useUniverse } from "../../composables/useUniverse";
import { useDashboardSections } from "../../composables/useDashboardSections";
import { onLogoError } from "@/branding";
import { UNIVERSES, type UniverseKey } from "@/universes";

const route = useRoute();
const router = useRouter();
const { toggle: toggleSidebar } = useSidebar();
const { user, logout, avatarUrl } = useAuth();
const { unreadCount, togglePanel } = useNotifications();
const { connected: wsConnected } = useRealtime();
const { guilds, selectedGuildId, fetchGuilds, selectGuild } = useGuildSelector();

/// Installation mono-serveur : la guilde vient de la configuration.
const guildeImposee = computed(() => !!siteConfig().guildId);

const { universe, definition, setUniverse, homePath } = useUniverse();
const { availableUniverses } = useDashboardSections();

/// La marque de la barre suit l'univers courant : on sait toujours chez quel
/// produit on se trouve, jamais dans un entre-deux ambigu.
const brand = computed(() => definition.value.brand);

/// Bascule d'univers : on navigue vers la page d'accueil DECLAREE par la
/// cible, plutot que de rester sur une route qui n'existe pas ailleurs.
function switchUniverse(target: UniverseKey) {
  if (target === universe.value) return;
  setUniverse(target);
  router.push(UNIVERSES[target].home);
}

function onGuildChange(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  selectGuild(value === "" ? null : value);
}

async function handleLogout() {
  await logout();
  router.push("/login");
}

/// Le logo ramene a l'accueil de L'UNIVERS COURANT. Il pointait auparavant en
/// dur sur /dashboard, ce qui faisait sortir de Nexus au lieu d'y revenir.
function goHome() {
  if (route.path !== homePath.value) router.push(homePath.value);
}

onMounted(() => {
  // Defense en profondeur : ne fetch pas si pas d'utilisateur logge
  // (sinon 401 parasite qui peut purger le token d'une session voisine).
  if (user.value) fetchGuilds();
});
</script>

<template>
  <header class="topbar">
    <div class="topbar-gloss" aria-hidden="true"></div>
    <button
      class="menu-btn"
      type="button"
      title="Menu"
      aria-label="Ouvrir le menu"
      @click="toggleSidebar"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="3" y1="6" x2="21" y2="6" />
        <line x1="3" y1="12" x2="21" y2="12" />
        <line x1="3" y1="18" x2="21" y2="18" />
      </svg>
    </button>
    <button class="brand" type="button" title="Accueil" @click="goHome">
      <span class="brand-halo" aria-hidden="true"></span>
      <img :src="brand.mark" :alt="brand.name" class="logo-icon" @error="onLogoError" />
      <span class="logo-text">{{ brand.name }}</span>
    </button>

    <div class="spacer" />

    <!-- Retour vers le site public. Sans ces liens, un administrateur
         connecte n'avait aucun moyen d'atteindre l'espace membre ni les
         jeux : le back-office etait un cul-de-sac. -->
    <RouterLink to="/membre" class="side-link" title="L'espace membre du site">
      🛋️ Espace membre
    </RouterLink>
    <RouterLink to="/jeux" class="side-link" title="Les jeux de la communaute">
      🎡 Jeux
    </RouterLink>

    <!-- Bascule d'univers, engendree par le registre : ajouter un univers ne
         demande aucune modification ici. Seuls apparaissent ceux qui ont au
         moins une entree de menu visible — proposer un univers qui n'amene
         que sur une barre laterale vide serait une fausse promesse. -->
    <div v-if="availableUniverses.length > 1" class="universe-switch">
      <button
        v-for="u in availableUniverses"
        :key="u.key"
        type="button"
        class="universe-btn"
        :class="{ active: universe === u.key }"
        :style="{ '--u-accent': u.accent }"
        :title="u.brand.tagline"
        @click="switchUniverse(u.key)"
      >
        {{ u.brand.name }}
      </button>
    </div>

    <!-- Masque en mono-serveur : proposer un choix qui n'en est pas un
         laisse croire qu'on gere plusieurs serveurs. -->
    <div v-if="!guildeImposee" class="guild-selector">
      <select
        class="guild-select"
        :value="selectedGuildId ?? ''"
        @change="onGuildChange"
      >
        <option value="">Tous les serveurs</option>
        <option v-for="g in guilds" :key="g.guild_id" :value="g.guild_id">
          {{ g.name }}
        </option>
      </select>
    </div>

    <div class="status-indicator" :title="wsConnected ? 'Connecte' : 'Deconnecte'">
      <StatusDot :status="wsConnected ? 'online' : 'offline'" />
    </div>

    <button class="bell-btn" title="Notifications" @click="togglePanel">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9" />
        <path d="M13.73 21a2 2 0 01-3.46 0" />
      </svg>
      <span v-if="unreadCount > 0" class="bell-badge">{{ unreadCount }}</span>
    </button>

    <div v-if="user" class="user-block">
      <img :src="avatarUrl(user)" :alt="user.username" class="user-avatar" />
      <div class="user-info">
        <span class="user-name">{{ user.global_name ?? user.username }}</span>
        <span class="user-tag">{{ user.username }}</span>
      </div>
      <button class="logout-btn" title="Deconnexion" @click="handleLogout">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9 21H5a2 2 0 01-2-2V5a2 2 0 012-2h4" />
          <polyline points="16 17 21 12 16 7" />
          <line x1="21" y1="12" x2="9" y2="12" />
        </svg>
      </button>
    </div>

    <NotificationModal />
  </header>
</template>

<style scoped src="../../styles/top-bar.css"></style>
