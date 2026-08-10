<script setup lang="ts">
import { useRoute } from "vue-router";
import SectionIcon from "../atoms/SectionIcon.vue";
import { useDashboardSections } from "../../composables/useDashboardSections";
import { useSidebar } from "../../composables/useSidebar";
import { useUniverse } from "../../composables/useUniverse";

const route = useRoute();
// La barre laterale ne montre que l'univers courant.
const { universe, definition, homePath } = useUniverse();
const { groups } = useDashboardSections(universe);
const { open, close, isCollapsed, toggleGroup } = useSidebar();

// L'accent vient de l'UNIVERS, plus d'une couleur par groupe de menu, et il
// est pose par `MainLayout` sur la coque (`--universe-accent`) : la barre
// laterale n'a qu'a le consommer.
//
// Avant, chaque groupe (moderation, communaute, securite…) avait sa teinte :
// la barre laterale etait donc aussi bariolee chez Sentinel que chez Nexus, et
// la couleur ne disait rien du produit dans lequel on se trouvait. Elle sert
// desormais a repondre a une seule question, la plus utile : ou suis-je ?

// Un lien est actif si la route courante commence par son path (pour que les
// hubs a onglets — /moderation — restent surlignes sur leurs sous-vues).
function isActive(path: string): boolean {
  // L'accueil de l'univers doit correspondre exactement : sinon il resterait
  // surligne sur toutes ses sous-routes.
  if (path === homePath.value) return route.path === homePath.value;
  return route.path === path || route.path.startsWith(path + "/");
}

// Sur mobile, cliquer un lien ferme le drawer.
function onNavigate() {
  close();
}
</script>

<template>
  <!-- Overlay mobile : ferme le drawer au clic hors sidebar. -->
  <div
    v-if="open"
    class="sidebar-overlay"
    aria-hidden="true"
    @click="close"
  ></div>

  <aside
    class="sidebar"
    :class="{ 'is-open': open }"
  >
    <nav class="sidebar-nav" aria-label="Navigation principale">
      <router-link
        :to="homePath"
        class="nav-item nav-home"
        :class="{ active: isActive(homePath) }"
        @click="onNavigate"
      >
        <span class="nav-icon"><SectionIcon name="grid" /></span>
        <span class="nav-label">{{ definition.brand.name }}</span>
      </router-link>

      <div v-for="g in groups" :key="g.prefix" class="nav-group">
        <button
          type="button"
          class="group-header"
          :aria-expanded="!isCollapsed(g.prefix)"
          @click="toggleGroup(g.prefix)"
        >
          <span class="group-label">{{ g.label }}</span>
          <svg
            class="group-chevron"
            :class="{ collapsed: isCollapsed(g.prefix) }"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>

        <div v-show="!isCollapsed(g.prefix)" class="group-items">
          <router-link
            v-for="s in g.sections"
            :key="s.key"
            :to="s.path"
            class="nav-item"
            :class="{ active: isActive(s.path) }"
            @click="onNavigate"
          >
            <span class="nav-icon"><SectionIcon :name="s.icon" /></span>
            <span class="nav-label">{{ s.label }}</span>
          </router-link>
        </div>
      </div>
    </nav>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 240px;
  flex-shrink: 0;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  overflow-y: auto;
  overflow-x: hidden;
  padding: 12px 8px 24px;
  user-select: none;
}

.sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-group {
  margin-top: 8px;
}

.group-header {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  padding: 6px 10px 4px;
  background: none;
  border: none;
  cursor: pointer;
  color: var(--text-secondary);
}
.group-label {
  font-size: 10.5px;
  font-weight: 700;
  letter-spacing: 0.6px;
  text-transform: uppercase;
  color: color-mix(in srgb, var(--universe-accent) 75%, var(--text-secondary));
}
.group-chevron {
  width: 13px;
  height: 13px;
  color: var(--text-secondary);
  opacity: 0.6;
  transition: transform 0.2s ease;
}
.group-chevron.collapsed {
  transform: rotate(-90deg);
}

.group-items {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: var(--radius-md);
  color: var(--text-secondary);
  text-decoration: none;
  font-size: 13px;
  font-weight: 500;
  border-left: 2px solid transparent;
  transition: background-color 0.15s ease, color 0.15s ease,
    border-color 0.15s ease;
}
.nav-item:hover {
  background-color: color-mix(in srgb, var(--universe-accent, var(--accent)) 12%, transparent);
  color: var(--text-primary);
}
.nav-item.active {
  background-color: color-mix(in srgb, var(--universe-accent, var(--accent)) 16%, transparent);
  color: var(--text-primary);
  border-left-color: var(--universe-accent, var(--accent));
  font-weight: 600;
}

/* Pas de surcharge d'accent ici : le lien d'accueil porte la couleur de son
   univers comme les autres, c'est justement lui qui l'annonce. */
.nav-home {
  margin-bottom: 4px;
}

.nav-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--universe-accent, var(--accent));
}
.nav-icon :deep(svg) {
  width: 16px;
  height: 16px;
}

.nav-label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sidebar-overlay {
  display: none;
}

/* ── Mobile : sidebar en drawer off-canvas ── */
@media (max-width: 900px) {
  .sidebar {
    position: fixed;
    top: 0;
    left: 0;
    bottom: 0;
    z-index: 50;
    transform: translateX(-100%);
    transition: transform 0.25s ease;
    box-shadow: 4px 0 24px rgba(0, 0, 0, 0.35);
  }
  .sidebar.is-open {
    transform: translateX(0);
  }
  .sidebar-overlay {
    display: block;
    position: fixed;
    inset: 0;
    z-index: 40;
    background: rgba(0, 0, 0, 0.5);
  }
}

@media (prefers-reduced-motion: reduce) {
  .sidebar,
  .group-chevron {
    transition: none;
  }
}
</style>
