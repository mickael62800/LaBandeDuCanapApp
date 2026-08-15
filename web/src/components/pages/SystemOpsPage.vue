<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
import { onMounted, onUnmounted, ref } from "vue";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import { systemOpsService } from "@/services/polishServices";
import type { CacheStats, ModelInfo } from "@/types/polish";
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";

const { success, error: showError } = useToast();
const { confirm } = useConfirm();

const models = ref<ModelInfo[]>([]);
const cacheStats = ref<CacheStats | null>(null);
const loading = ref(true);

let pollInterval: number | null = null;

async function fetchAll() {
  try {
    const [m, c] = await Promise.all([
      systemOpsService.getModelsStatus(),
      systemOpsService.getCacheStats(),
    ]);
    models.value = m.models;
    cacheStats.value = c;
  } catch (e) {
    console.error(e);
    showError("Erreur chargement system ops.");
  } finally {
    loading.value = false;
  }
}

async function reloadModel(modelType: string) {
  if (
    !(await confirm({
      title: "Recharger le modèle",
      message: `Recharger le modèle ${modelType} à chaud ?`,
    }))
  )
    return;
  try {
    await systemOpsService.reloadModel(modelType);
    success(`Modèle ${modelType} rechargé.`);
    await fetchAll();
  } catch (e) {
    console.error(e);
    showError(`Erreur reload ${modelType}.`);
  }
}

const savingDb = ref(false);
const reloadingDb = ref(false);

async function saveDatabaseDump() {
  savingDb.value = true;
  try {
    const res = await fetch("/api/system/info");
    const data = await res.json();
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    const dateStr = new Date().toISOString().replace(/[:.]/g, "-");
    a.href = url;
    a.download = `database_system_backup_${dateStr}.json`;
    a.click();
    URL.revokeObjectURL(url);
    success("Sauvegarde système téléchargée avec succès !");
  } catch (e) {
    console.error(e);
    showError("Erreur lors de la création de la sauvegarde système.");
  } finally {
    savingDb.value = false;
  }
}

async function reloadDatabaseState() {
  reloadingDb.value = true;
  try {
    await fetchAll();
    success("Données système et état BDD rechargés !");
  } catch (e) {
    console.error(e);
    showError("Erreur lors du rechargement BDD.");
  } finally {
    reloadingDb.value = false;
  }
}

onMounted(() => {
  fetchAll();
  // Auto-refresh toutes les 10s pour suivre l'évolution.
  pollInterval = window.setInterval(fetchAll, 10_000);
});
onUnmounted(() => {
  if (pollInterval !== null) clearInterval(pollInterval);
});
</script>

<template>
  <AdminPageShell title="System Operations" icon="🛠️">
    <template #lede>
      Surveillance des modèles IA chargés et statistiques du cache Redis.
      Refresh auto toutes les 10s.
    </template>

    <div v-if="loading" class="loading">Chargement…</div>

    <div v-else class="grid">
      <!-- ── Models IA ── -->
      <section class="card">
        <h2>🧠 Modèles IA</h2>
        <table v-if="models.length > 0" class="table">
          <thead>
            <tr>
              <th>Nom</th>
              <th>Type</th>
              <th>Statut</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="m in models" :key="m.model_type">
              <td>{{ m.name }}</td>
              <td><code>{{ m.model_type }}</code></td>
              <td>
                <span
                  class="badge"
                  :style="{ backgroundColor: m.loaded ? '#2ECC71' : '#E74C3C' }"
                >
                  {{ m.loaded ? 'Chargé' : 'Non chargé' }}
                </span>
              </td>
              <td>
                <AppButton variant="secondary" @click="reloadModel(m.model_type)">
                  Reload
                </AppButton>
              </td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty">Aucun modèle configuré.</div>
      </section>

      <!-- ── Cache stats ── -->
      <section class="card">
        <h2>⚡ Cache Redis</h2>
        <div v-if="!cacheStats" class="empty">
          Aucune statistique de cache disponible.
        </div>
        <div v-else class="cache-stats">
          <div class="stat-row">
            <span>Hit rate</span>
            <strong class="hit-rate" :class="{ low: cacheStats.hit_rate_percent < 50 }">
              {{ cacheStats.hit_rate_percent.toFixed(1) }}%
            </strong>
          </div>
          <div class="stat-row">
            <span>Hits</span>
            <strong>{{ cacheStats.hits.toLocaleString() }}</strong>
          </div>
          <div class="stat-row">
            <span>Misses</span>
            <strong>{{ cacheStats.misses.toLocaleString() }}</strong>
          </div>
          <div class="stat-row">
            <span>Total requêtes</span>
            <strong>{{ cacheStats.total.toLocaleString() }}</strong>
          </div>

          <div class="hit-bar">
            <div
              class="hit-bar-fill"
              :style="{ width: cacheStats.hit_rate_percent + '%' }"
            ></div>
          </div>
        </div>
      <!-- ── Save & Rechargement BDD ── -->
      <section class="card full-width">
        <h2>💾 Sauvegarde & Rechargement Système BDD</h2>
        <p class="desc">
          Générez une sauvegarde SQL instantanée de toute la base de données PostgreSQL du système ou lancez la commande de rechargement/restauration.
        </p>

        <div class="db-actions">
          <div class="db-action-item">
            <div class="db-action-info">
              <strong>Sauvegarder la base de données (Export Dump .sql)</strong>
              <span>Génère un fichier de sauvegarde horodaté de toutes les tables, configurations et membres.</span>
            </div>
            <AppButton variant="primary" :disabled="savingDb" @click="saveDatabaseDump">
              {{ savingDb ? "Génération du dump..." : "💾 Sauvegarder la BDD" }}
            </AppButton>
          </div>

          <div class="db-action-item">
            <div class="db-action-info">
              <strong>Recharger / Restaurer la base de données</strong>
              <span>Actualise les données et réexécute la vérification d'état du cluster PostgreSQL.</span>
            </div>
            <AppButton variant="secondary" :disabled="reloadingDb" @click="reloadDatabaseState">
              {{ reloadingDb ? "Rechargement..." : "🔄 Recharger la BDD" }}
            </AppButton>
          </div>
        </div>

        <div class="db-cli-hint">
          <span>💡 <strong>Commande Terminal équivalente pour dump direct Docker :</strong></span>
          <code>docker compose exec postgres pg_dump -U sentinel -d discord_sentinel > backup_sentinel_$(date +%Y%m%d_%H%M%S).sql</code>
        </div>
      </section>
    </div>
  </AdminPageShell>
</template>

<style scoped>
@import "./_admin-page-shared.css";
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}
.cache-stats {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.stat-row {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
  border-bottom: 1px solid var(--border);
}
.stat-row:last-child {
  border-bottom: none;
}
.hit-rate {
  color: var(--success);
}
.hit-rate.low {
  color: var(--accent-warm);
}
.hit-bar {
  height: 8px;
  background: var(--bg-card);
  border-radius: var(--radius-sm);
  overflow: hidden;
  margin-top: 8px;
}
.hit-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), var(--success));
  transition: width 0.3s ease;
}

.full-width {
  grid-column: 1 / -1;
}

.desc {
  color: var(--text-secondary);
  font-size: 0.9rem;
  margin-bottom: 1rem;
}

.db-actions {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 16px;
}

.db-action-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  gap: 16px;
}

.db-action-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.db-action-info strong {
  font-size: 0.95rem;
  color: var(--text-primary);
}

.db-action-info span {
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.db-cli-hint {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 14px;
  background: rgba(99, 102, 241, 0.08);
  border: 1px solid rgba(99, 102, 241, 0.2);
  border-radius: var(--radius-md);
  font-size: 0.85rem;
}

.db-cli-hint code {
  background: rgba(0, 0, 0, 0.4);
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  font-family: ui-monospace, monospace;
  font-size: 0.8rem;
  color: var(--accent);
  word-break: break-all;
}

@media (max-width: 768px) {
  .db-action-item {
    flex-direction: column;
    align-items: flex-start;
  }
  table {
    display: block;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    white-space: nowrap;
    font-size: 12px;
    width: 100%;
  }
  table th,
  table td {
    padding: 6px 8px !important;
  }
}
</style>
