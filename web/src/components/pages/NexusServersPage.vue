<script setup lang="ts">
// Serveurs de jeu de la plateforme Nexus : etat de la flotte + pilotage.
//
// Les actions (demarrer / arreter / redemarrer) passent par la passerelle
// /nexus-api, elle-meme gardee par le gate RBAC `nexus.access`. Le front ne
// fait donc pas de controle de droits : il refleterait un etat qu'il ne peut
// pas garantir. Un refus remonte en 403 et est affiche tel quel.

import { computed, ref, watch } from "vue";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useToast } from "../../composables/useToast";
import {
  nexusGamesService,
  adresseServeur,
  type GameServer,
  type GameTemplate,
} from "@/services/nexusGamesService";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import DashboardHero from "../organisms/DashboardHero.vue";

const { selectedGuildId, selectedGuild } = useGuildSelector();
// Idem `NexusServerDetailPage` : l'acteur de l'audit vient de la passerelle.
const { success, error: showError } = useToast();

const servers = ref<GameServer[]>([]);
const templates = ref<GameTemplate[]>([]);
const loading = ref(false);
const errorMessage = ref("");
/// Id du serveur dont une action est en cours (desactive ses boutons).
const busyId = ref<string | null>(null);

/// Copie l'adresse pour la coller dans le jeu. `writeText` echoue hors HTTPS
/// ou sans autorisation : on le dit plutot que de laisser croire au succes.
async function copier(adresse: string) {
  try {
    await navigator.clipboard.writeText(adresse);
    success(`Adresse copiee : ${adresse}`);
  } catch {
    showError("Copie impossible, selectionne l'adresse a la main");
  }
}

const templateName = computed(() => {
  const byId = new Map(templates.value.map((t) => [t.id, t.name]));
  return (id: string) => byId.get(id) ?? "Jeu inconnu";
});

/// Libelle FR + classe CSS par etat, pour eviter d'afficher les valeurs brutes.
const STATUS_LABELS: Record<string, string> = {
  created: "Cree",
  scheduled: "En attente d'ouverture",
  starting: "Demarrage…",
  running: "En ligne",
  stopping: "Arret…",
  stopped: "Arrete",
  error: "Erreur",
  deleted: "Supprime",
};

/// Un serveur en transition ne doit pas accepter de nouvelle action.
function isTransient(s: GameServer): boolean {
  return s.status === "starting" || s.status === "stopping";
}

async function load() {
  if (!selectedGuildId.value) {
    servers.value = [];
    return;
  }
  loading.value = true;
  errorMessage.value = "";
  try {
    const [s, t] = await Promise.all([
      nexusGamesService.listServers(selectedGuildId.value),
      nexusGamesService.listTemplates(selectedGuildId.value).catch(() => [] as GameTemplate[]),
    ]);
    servers.value = s;
    templates.value = t;
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Chargement impossible";
    servers.value = [];
  } finally {
    loading.value = false;
  }
}

async function act(server: GameServer, action: "start" | "stop" | "restart") {
  if (!selectedGuildId.value || busyId.value) return;
  busyId.value = server.id;
  try {
    await nexusGamesService[action](selectedGuildId.value, server.id);
    success(`${server.name} : action envoyee`);
    // L'API repond des que l'ordre est pris en compte, mais le conteneur met
    // quelques secondes a changer d'etat : on recharge apres un court delai.
    setTimeout(load, 1500);
  } catch (e) {
    showError(e instanceof Error ? e.message : "Action impossible");
  } finally {
    busyId.value = null;
  }
}

watch(selectedGuildId, load, { immediate: true });
</script>

<template>
  <AdminPageShell
    title="Serveurs de jeu"
    :subtitle="selectedGuild?.name ?? 'Aucun serveur selectionne'"
  >
    <DashboardHero
      title="Nexus Games"
      subtitle="Plateforme d'hébergement et de pilotage de vos serveurs de jeu."
      logo="/nexus_logo.png"
      universe="nexus"
    />

    <p v-if="!selectedGuildId" class="ns-hint">
      Selectionne un serveur Discord pour voir sa flotte.
    </p>

    <p v-else-if="errorMessage" class="ns-error">{{ errorMessage }}</p>

    <p v-else-if="loading" class="ns-hint">Chargement…</p>

    <template v-else>
      <RouterLink to="/nexus/servers/nouveau" class="ns-new">+ Nouveau serveur</RouterLink>

    <p v-if="!servers.length" class="ns-hint">
      Aucun serveur de jeu pour l'instant. Cree-en un ci-dessus, ou depuis
      Discord avec <code>/game-admin</code>.
    </p>

    <table v-else class="ns-table">
      <thead>
        <tr>
          <th>Nom</th>
          <th>Jeu</th>
          <th>Etat</th>
          <th>Joueurs</th>
          <th>Adresse</th>
          <th>Memoire</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="s in servers" :key="s.id">
          <td>
            <RouterLink :to="`/nexus/servers/${s.id}`" class="ns-name">{{ s.name }}</RouterLink>
            <span v-if="s.last_error" class="ns-lasterror" :title="s.last_error">⚠</span>
          </td>
          <td>{{ templateName(s.template_id) }}</td>
          <td>
            <span class="ns-status" :class="`st-${s.status}`">
              {{ STATUS_LABELS[s.status] ?? s.status }}
            </span>
          </td>
          <td>{{ s.last_player_count }}</td>
          <td class="ns-adresse">
            <button
              v-if="adresseServeur(s)"
              type="button"
              class="ns-copie"
              :title="`Copier ${adresseServeur(s)}`"
              @click="copier(adresseServeur(s)!)"
            >
              {{ adresseServeur(s) }}
            </button>
            <span v-else-if="s.host_port" class="ns-partiel" title="Hote public non configure">
              :{{ s.host_port }}
            </span>
            <span v-else>—</span>
          </td>
          <td>{{ s.allocated_memory_mb }} Mo</td>
          <td class="ns-actions">
            <button
              v-if="s.status !== 'running'"
              type="button"
              :disabled="busyId === s.id || isTransient(s)"
              @click="act(s, 'start')"
            >
              Demarrer
            </button>
            <button
              v-else
              type="button"
              :disabled="busyId === s.id"
              @click="act(s, 'stop')"
            >
              Arreter
            </button>
            <button
              type="button"
              :disabled="busyId === s.id || isTransient(s)"
              @click="act(s, 'restart')"
            >
              Redemarrer
            </button>
          </td>
        </tr>
      </tbody>
    </table>
    </template>
  </AdminPageShell>
</template>

<style scoped>
.ns-hint {
  color: var(--text-secondary);
}

.ns-error {
  color: var(--danger);
}

.ns-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.92rem;
}

.ns-table th,
.ns-table td {
  text-align: left;
  padding: var(--space-sm);
  border-bottom: 1px solid var(--bg-hover);
}

.ns-table th {
  color: var(--text-secondary);
  font-weight: 600;
}

.ns-name {
  color: var(--text-primary);
  font-weight: 600;
}

.ns-name:hover {
  color: var(--accent);
}

.ns-new {
  display: inline-block;
  margin-bottom: var(--space-md);
  padding: 6px 16px;
  background: var(--accent);
  color: #fff;
  border-radius: var(--radius-md);
  font-size: 0.9rem;
}

.ns-adresse {
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

/* Bouton discret : l'adresse reste lisible comme du texte, mais se copie. */
.ns-copie {
  background: none;
  border: none;
  padding: 0;
  font: inherit;
  color: var(--text-primary);
  cursor: pointer;
  border-bottom: 1px dashed var(--border-subtle);
}

.ns-copie:hover {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.ns-partiel {
  color: var(--text-muted);
}

.ns-lasterror {
  margin-left: 6px;
  color: var(--warning);
  cursor: help;
}

.ns-status {
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  font-size: 0.82rem;
  background: var(--bg-card);
  color: var(--text-secondary);
}

.st-running {
  background: color-mix(in srgb, var(--success) 20%, transparent);
  color: var(--success);
}

.st-error {
  background: color-mix(in srgb, var(--danger) 20%, transparent);
  color: var(--danger);
}

.st-starting,
.st-stopping {
  background: color-mix(in srgb, var(--warning) 20%, transparent);
  color: var(--warning);
}

.ns-actions {
  display: flex;
  gap: var(--space-xs);
}

.ns-actions button {
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  color: var(--text-primary);
  border-radius: var(--radius-sm);
  padding: 4px 10px;
  cursor: pointer;
  transition: var(--transition-fast);
}

.ns-actions button:hover:not(:disabled) {
  border-color: var(--accent);
}

.ns-actions button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
