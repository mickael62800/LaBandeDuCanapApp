<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import StatusDot from "../atoms/StatusDot.vue";
import NotificationPanel from "./NotificationPanel.vue";
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
const { unreadCount, panelOpen, togglePanel } = useNotifications();
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

    <NotificationPanel v-if="panelOpen" />
  </header>
</template>

<style scoped>
.topbar {
  position: relative;
  overflow: hidden;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 20px;
  /* Mesh gradient discret en background : 2 radial-gradients qui ajoutent
     de la couleur au bg-secondary sans le surcharger. */
  background:
    radial-gradient(ellipse at 0% 50%,
      color-mix(in srgb, var(--accent) 10%, transparent) 0%,
      transparent 40%),
    radial-gradient(ellipse at 100% 50%,
      color-mix(in srgb, var(--accent-alt, #a855f7) 8%, transparent) 0%,
      transparent 40%),
    var(--bg-secondary);
  border-bottom: 1px solid transparent;
  user-select: none;
  flex-shrink: 0;
}

/* Bordure inferieure : ligne en gradient lumineux qui remplace le
   border-bottom plat, avec un legere pulsation au centre. */
.topbar::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 1px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    color-mix(in srgb, var(--accent) 50%, var(--border)) 20%,
    color-mix(in srgb, var(--accent) 80%, var(--border)) 50%,
    color-mix(in srgb, var(--accent-alt, #a855f7) 50%, var(--border)) 80%,
    transparent 100%
  );
  pointer-events: none;
}

/* Gloss periodique : un balayage discret toutes les 14s sur la topbar. */
.topbar-gloss {
  position: absolute;
  top: -50%;
  left: -50%;
  width: 25%;
  height: 200%;
  background: linear-gradient(
    115deg,
    transparent 0%,
    color-mix(in srgb, white 0%, transparent) 40%,
    color-mix(in srgb, white 12%, transparent) 50%,
    color-mix(in srgb, white 0%, transparent) 60%,
    transparent 100%
  );
  transform: skewX(-20deg);
  pointer-events: none;
  animation: topbar-gloss-loop 14s ease-out 1s infinite;
  z-index: 0;
}
@keyframes topbar-gloss-loop {
  0%   { left: -50%; }
  10%  { left: 150%; }
  100% { left: 150%; }
}

/* Bouton hamburger : ouvre le drawer de navigation. Masque sur desktop
   (la sidebar y est toujours visible), affiche sur mobile <=900px. */
.menu-btn {
  display: none;
  width: 34px;
  height: 34px;
  padding: 7px;
  background: none;
  border-radius: var(--radius-md);
  color: var(--text-secondary);
  align-items: center;
  justify-content: center;
  z-index: 1;
  flex-shrink: 0;
  transition: background-color 0.2s ease, color 0.2s ease;
}
.menu-btn:hover {
  background-color: color-mix(in srgb, var(--accent) 12%, transparent);
  color: var(--accent);
}
.menu-btn svg {
  width: 20px;
  height: 20px;
}
@media (max-width: 900px) {
  .menu-btn {
    display: flex;
  }
}

.brand {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  background: none;
  padding: 4px 10px;
  border-radius: var(--radius-md);
  border: 1px solid color-mix(in srgb, var(--accent) 30%, var(--border));
  z-index: 1;
  transition: background-color 0.2s ease,
    border-color 0.25s ease,
    transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1),
    box-shadow 0.25s ease;
}
.brand:hover {
  background-color: color-mix(in srgb, var(--accent) 10%, transparent);
  border-color: color-mix(in srgb, var(--accent) 70%, var(--border));
  box-shadow: 0 2px 10px color-mix(in srgb, var(--accent) 25%, transparent);
  transform: scale(1.03);
}

/* Halo discret derriere le bouton brand : pulse leger pour signaler
   l'identite "vivante" de l'app. */
.brand-halo {
  position: absolute;
  inset: -4px;
  border-radius: var(--radius-lg);
  background: radial-gradient(circle at 18px 50%,
    color-mix(in srgb, var(--accent) 40%, transparent) 0%,
    transparent 60%);
  opacity: 0.55;
  filter: blur(6px);
  pointer-events: none;
  z-index: -1;
  animation: brand-halo-pulse 4s ease-in-out infinite;
}
@keyframes brand-halo-pulse {
  0%, 100% { opacity: 0.4; }
  50%      { opacity: 0.75; }
}

.logo-icon {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  object-fit: contain;
  filter: drop-shadow(0 2px 6px color-mix(in srgb, var(--accent) 40%, transparent));
  transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.brand:hover .logo-icon {
  transform: rotate(-6deg) scale(1.05);
}

.logo-text {
  font-size: 16px;
  font-weight: 700;
  /* Gradient text avec shimmer leger : moins agressif que le hero. */
  background: linear-gradient(
    90deg,
    var(--text-primary) 0%,
    color-mix(in srgb, var(--accent) 70%, var(--text-primary)) 50%,
    var(--text-primary) 100%
  );
  background-size: 200% auto;
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  color: transparent;
  animation: brand-shimmer 8s linear infinite;
  letter-spacing: 0.3px;
}
@keyframes brand-shimmer {
  0%   { background-position: 200% center; }
  100% { background-position: -200% center; }
}

.spacer {
  flex: 1;
}

.guild-selector { position: relative; z-index: 1; }
.status-indicator { position: relative; z-index: 1; }
.user-block { position: relative; z-index: 1; }

.guild-select {
  padding: 7px 28px 7px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23888' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  min-width: 180px;
}
.guild-select:hover {
  border-color: var(--accent);
}
.guild-select:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
}

.status-indicator {
  display: flex;
  align-items: center;
  padding: 0 4px;
}

.bell-btn {
  position: relative;
  width: 34px;
  height: 34px;
  padding: 7px;
  background: none;
  border-radius: var(--radius-md);
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color 0.2s ease, color 0.2s ease, transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  z-index: 1;
}
.bell-btn:hover {
  background-color: color-mix(in srgb, var(--accent) 12%, transparent);
  color: var(--accent);
  transform: scale(1.08);
}
.bell-btn svg {
  width: 18px;
  height: 18px;
}
.bell-badge {
  position: absolute;
  top: 2px;
  right: 2px;
  min-width: 16px;
  height: 16px;
  border-radius: var(--radius-md);
  background-color: var(--danger);
  color: white;
  font-size: 10px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 4px;
}

.user-block {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-left: 12px;
  margin-left: 4px;
  border-left: 1px solid var(--border);
}

.user-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  flex-shrink: 0;
}

.user-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  max-width: 140px;
}

.user-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.user-tag {
  font-size: 11px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.logout-btn {
  width: 30px;
  height: 30px;
  padding: 5px;
  background: none;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
}
.logout-btn:hover {
  background-color: var(--bg-hover);
  color: var(--danger);
}
.logout-btn svg {
  width: 16px;
  height: 16px;
}

@media (max-width: 700px) {
  .user-info { display: none; }
  .logo-text { display: none; }
  .topbar {
    padding: 8px 10px;
    gap: 8px;
  }
  .guild-select {
    min-width: 0;
    max-width: 160px;
    padding: 6px 28px 6px 10px;
    font-size: 12px;
  }
  .brand { flex-shrink: 0; padding: 0; }
  .logo-icon { width: 36px; height: 36px; }
  .user-block {
    padding-left: 8px;
    margin-left: 0;
    border-left: 1px solid var(--border);
    flex-shrink: 0;
  }
  .user-avatar { width: 28px; height: 28px; }
  .bell-btn { width: 32px; height: 32px; }
  /* Spacer compressible pour laisser la place a l'avatar user. */
  .spacer { min-width: 0; flex-shrink: 1; }
}

/* Tres petit mobile : encore plus compact */
@media (max-width: 420px) {
  .topbar {
    padding: 6px 8px;
    gap: 6px;
  }
  .guild-select {
    max-width: 130px;
    font-size: 11px;
  }
  .status-indicator { display: none; }
}

@media (prefers-reduced-motion: reduce) {
  .topbar-gloss { display: none; }
  .brand-halo { animation: none; opacity: 0.5; }
  .logo-text {
    animation: none;
    background: none;
    -webkit-text-fill-color: var(--text-primary);
    color: var(--text-primary);
  }
  .brand,
  .brand:hover,
  .bell-btn:hover { transform: none; }
}

.universe-switch {
  display: flex;
  gap: 2px;
  background: var(--bg-card);
  border-radius: var(--radius-md);
  padding: 2px;
}

.universe-btn {
  background: transparent;
  border: none;
  color: var(--text-secondary);
  font-size: 0.85rem;
  padding: 4px 12px;
  border-radius: calc(var(--radius-md) - 2px);
  cursor: pointer;
  transition: var(--transition-fast);
}

.universe-btn:hover {
  color: var(--text-primary);
}

/* Chaque univers porte SA couleur, y compris au repos (pastille) : la
   bascule devient lisible d'un coup d'oeil au lieu d'un simple onglet actif. */
.universe-btn::before {
  content: "";
  display: inline-block;
  width: 7px;
  height: 7px;
  margin-right: 6px;
  border-radius: 50%;
  background: var(--u-accent, var(--accent));
  vertical-align: middle;
}

.universe-btn.active {
  background: var(--u-accent, var(--accent));
  color: #fff;
}
.universe-btn.active::before {
  background: #fff;
}

@media (max-width: 700px) {
  .universe-switch {
    display: none;
  }
}

.side-link {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.25rem 0.8rem;
  border-radius: var(--radius-pill);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 0.82rem;
  text-decoration: none;
  white-space: nowrap;
}

.side-link:hover {
  border-color: var(--accent);
  color: var(--text-primary);
}

@media (max-width: 900px) {
  /* Sur petit ecran la barre est deja chargee : on garde l'emoji seul. */
  .side-link {
    font-size: 0;
    padding: 0.25rem 0.5rem;
    gap: 0;
  }
}
</style>
