<script setup lang="ts">
import IconButton from "../atoms/IconButton.vue";
import { useRolePanels } from "../../composables/useRolePanels";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useConfirm } from "../../composables/useConfirm";
import AppBadge from "../atoms/AppBadge.vue";

const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();
const { panels, autoRoles, selectedPanel, loading, selectPanel, deletePanel, removeAutoRole } =
  useRolePanels();

async function onDelete(panelId: string, title: string, ev: Event) {
  ev.stopPropagation();
  if (!(await confirm({ title: "Supprimer le panel", message: `Supprimer le panel "${title}" ?` })))
    return;
  await deletePanel(panelId);
}

async function onRemoveAutoRole(roleId: string, name: string) {
  if (
    !(await confirm({ title: "Retirer l'auto-rôle", message: `Retirer l'auto-rôle "${name}" ?` }))
  )
    return;
  if (!selectedGuildId.value) return;
  await removeAutoRole(selectedGuildId.value, roleId);
}

function styleColor(style: string): string {
  switch (style) {
    case "primary": return "#5865F2";
    case "secondary": return "var(--text-secondary)";
    case "success": return "var(--success)";
    case "danger": return "var(--danger)";
    default: return "#5865F2";
  }
}
</script>

<template>
  <!-- Contenu d'onglet : l'en-tete de page appartient a `RolesHubPage`.
       Le lien croise vers les roles Discord a disparu : l'onglet voisin fait
       exactement ce qu'il faisait, en mieux. -->
  <div class="role-panels">
    <div class="page-header-row">
      <router-link
        v-if="selectedGuildId"
        to="/role-panels/new"
        class="btn-primary"
      >+ Nouveau panel</router-link>
    </div>

    <div v-if="!selectedGuildId && !loading" class="empty">
      Selectionnez un serveur pour voir les panels de roles.
    </div>

    <div v-else-if="loading" class="loading">Chargement...</div>

    <template v-else>
      <!-- Auto-Roles -->
      <section v-if="autoRoles.length > 0" class="section">
        <h2>Auto-Roles (a l'arrivee)</h2>
        <div class="auto-roles-list">
          <div v-for="ar in autoRoles" :key="ar.id" class="auto-role-card">
            <div class="ar-info">
              <span class="ar-name">{{ ar.role_name || ar.role_id }}</span>
              <AppBadge :label="ar.enabled ? 'Actif' : 'Inactif'" :variant="ar.enabled ? 'success' : 'default'" />
            </div>
            <span v-if="ar.delay_secs > 0" class="ar-delay">Delai : {{ ar.delay_secs }}s</span>
            <span v-else class="ar-delay">Immediat</span>
            <IconButton label="Retirer cet auto-role" variant="danger" @click="onRemoveAutoRole(ar.role_id, ar.role_name || ar.role_id)">🗑️</IconButton>
          </div>
        </div>
      </section>

      <!-- Panels de roles -->
      <section class="section">
        <h2>Panels de roles ({{ panels.length }})</h2>

        <div v-if="panels.length === 0" class="empty">
          Aucun panel configure. Creez-en un depuis cette page ou utilisez /roles-panel deploy dans Discord.
        </div>

        <div class="panels-grid">
          <div
            v-for="panel in panels"
            :key="panel.id"
            :class="['card', 'panel-card', { selected: selectedPanel?.panel.id === panel.id }]"
            @click="selectPanel(panel.id)"
          >
            <div class="panel-header">
              <span class="panel-title">{{ panel.title }}</span>
              <AppBadge
                :label="panel.message_id ? 'Deploye' : 'Non deploye'"
                :variant="panel.message_id ? 'success' : 'warning'"
              />
              <IconButton label="Supprimer le panel" variant="danger" @click="onDelete(panel.id, panel.title, $event)">🗑️</IconButton>
            </div>
            <p v-if="panel.description" class="panel-desc">{{ panel.description }}</p>
            <div class="panel-meta">
              <span>Mode : {{ panel.mode }}</span>
              <span v-if="panel.max_roles">Max : {{ panel.max_roles }} roles</span>
              <span class="mono">{{ panel.channel_id }}</span>
            </div>
          </div>
        </div>
      </section>

      <!-- Detail du panel selectionne -->
      <section v-if="selectedPanel" class="section">
        <h2>{{ selectedPanel.panel.title }} — Roles</h2>
        <div class="entries-list">
          <div v-for="entry in selectedPanel.entries" :key="entry.id" class="entry-card">
            <div class="entry-button" :style="{ borderColor: styleColor(entry.style) }">
              <span v-if="entry.emoji" class="entry-emoji">{{ entry.emoji }}</span>
              <span class="entry-label">{{ entry.label }}</span>
            </div>
            <div class="entry-info">
              <span class="entry-role">{{ entry.role_name || entry.role_id }}</span>
              <span class="entry-style">{{ entry.style }}</span>
            </div>
          </div>
        </div>
      </section>
    </template>
  </div>
</template>

<style scoped>
.role-panels h1 { margin-bottom: 20px; }

.section { margin-bottom: 28px; }
.section h2 {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 14px;
  color: var(--text-primary);
}

/* Auto-Roles */
.auto-roles-list { display: flex; gap: 10px; flex-wrap: wrap; }

.auto-role-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 180px;
}

.ar-info { display: flex; align-items: center; gap: 8px; }
.ar-name { font-weight: 600; font-size: 14px; }
.ar-delay { font-size: 11px; color: var(--text-secondary); }

/* Panels grid */
.panels-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 12px; }

.panel-card {
  cursor: pointer;
  transition: all var(--transition-fast);
}
.panel-card:hover { border-color: var(--accent); }
.panel-card.selected { border-color: var(--accent); box-shadow: var(--focus-ring); }

.panel-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
.panel-title { font-weight: 600; font-size: 15px; }
.panel-desc { font-size: 13px; color: var(--text-secondary); margin-bottom: 8px; }
.panel-meta { display: flex; gap: 12px; font-size: 11px; color: var(--text-secondary); }
.mono { font-family: "JetBrains Mono", "Cascadia Code", monospace; }

/* Entries */
.entries-list { display: flex; gap: 10px; flex-wrap: wrap; }

.entry-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 150px;
}

.entry-button {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border: 2px solid;
  border-radius: var(--radius-md);
  background-color: var(--bg-secondary);
}
.entry-emoji { font-size: 16px; }
.entry-label { font-weight: 600; font-size: 13px; }
.entry-info { display: flex; justify-content: space-between; font-size: 11px; color: var(--text-secondary); }
.entry-role { font-weight: 500; }

.loading, .empty { color: var(--text-secondary); padding: 40px; text-align: center; }

/* Cross-link */
.page-header-row { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-header-row h1 { margin-bottom: 0; }
.cross-link { font-size: 13px; font-weight: 600; color: var(--accent); text-decoration: none; padding: 8px 16px; border: 1px solid var(--accent); border-radius: var(--radius-md); white-space: nowrap; transition: all var(--transition-fast); }
.cross-link:hover { background: var(--accent); color: white; }
.header-actions { display: flex; gap: 12px; align-items: center; }
.btn-primary { font-size: 13px; font-weight: 600; padding: 8px 16px; border: none; border-radius: var(--radius-md); background: var(--accent, var(--accent)); color: white; cursor: pointer; text-decoration: none; }
.btn-primary:hover { opacity: 0.9; }
.btn-icon-danger:hover { opacity: 1; }
</style>
