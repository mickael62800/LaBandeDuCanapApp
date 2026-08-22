<script setup lang="ts">
import { computed } from 'vue';

interface GameResourcesData {
  name: string;
  icon: string;
  recommendations: Array<{
    players: number;
    ram_gb: string;
    cpu_cores: number;
    notes: string;
  }>;
}

const games: GameResourcesData[] = [
  {
    name: 'Minecraft Java',
    icon: '⛏️',
    recommendations: [
      { players: 4, ram_gb: '4', cpu_cores: 2, notes: 'Vanilla, petite exploration' },
      { players: 10, ram_gb: '6-8', cpu_cores: 4, notes: 'Vanilla, exploration modérée' },
      { players: 20, ram_gb: '8-12', cpu_cores: 4, notes: 'Beaucoup de plugins/mods' },
    ],
  },
  {
    name: 'Valheim',
    icon: '🪓',
    recommendations: [
      { players: 5, ram_gb: '4', cpu_cores: 2, notes: 'Petit monde, peu exploré' },
      { players: 10, ram_gb: '6-8', cpu_cores: 4, notes: 'Monde établi, 10 joueurs max' },
    ],
  },
  {
    name: 'Factorio',
    icon: '⚙️',
    recommendations: [
      { players: 4, ram_gb: '3-4', cpu_cores: 2, notes: 'Usine petite/moyenne' },
      { players: 8, ram_gb: '4-6', cpu_cores: 4, notes: 'Usine établie' },
      { players: 10, ram_gb: '6-8', cpu_cores: 4, notes: 'Grosse usine ou Space Age' },
    ],
  },
  {
    name: 'Palworld',
    icon: '🐾',
    recommendations: [
      { players: 8, ram_gb: '8', cpu_cores: 4, notes: 'Small bases' },
      { players: 16, ram_gb: '16', cpu_cores: 4, notes: 'Recommandation officielle' },
      { players: 32, ram_gb: '24+', cpu_cores: 4, notes: 'Beaucoup de bases actives' },
    ],
  },
  {
    name: 'ARK: Survival Evolved',
    icon: '🦖',
    recommendations: [
      { players: 10, ram_gb: '8', cpu_cores: 2, notes: 'Vanilla, peu de mods' },
      { players: 15, ram_gb: '12-16', cpu_cores: 4, notes: 'Vanilla standard' },
      { players: 20, ram_gb: '16-20', cpu_cores: 4, notes: 'Mods et structures' },
    ],
  },
  {
    name: '7 Days to Die',
    icon: '🧟',
    recommendations: [
      { players: 4, ram_gb: '4-6', cpu_cores: 4, notes: 'Vanilla, 4 joueurs' },
      { players: 8, ram_gb: '8-12', cpu_cores: 6, notes: 'Avec mods légers' },
      { players: 16, ram_gb: '12-16', cpu_cores: 6, notes: 'Mods complets' },
    ],
  },
  {
    name: 'Terraria',
    icon: '🌳',
    recommendations: [
      { players: 3, ram_gb: '0.5-1', cpu_cores: 1, notes: 'Vanilla léger' },
      { players: 10, ram_gb: '1-2', cpu_cores: 2, notes: 'Plusieurs joueurs' },
      { players: 16, ram_gb: '2-4', cpu_cores: 4, notes: 'Avec mods' },
    ],
  },
];
</script>

<template>
  <div class="resources-guide">
    <div class="guide-header">
      <h3>💾 Recommandations de ressources</h3>
      <p class="guide-intro">
        Avant de créer ton serveur, utilise ce guide pour choisir la RAM et les CPU adaptés.
        Les valeurs sont <strong>indicatives</strong> et peuvent varier selon ta configuration.
      </p>
    </div>

    <div class="games-grid">
      <details v-for="game in games" :key="game.name" class="game-section" open>
        <summary class="game-header">
          <span class="game-icon">{{ game.icon }}</span>
          <span class="game-name">{{ game.name }}</span>
        </summary>

        <table class="recommendations-table">
          <thead>
            <tr>
              <th>Joueurs</th>
              <th>RAM</th>
              <th>CPU</th>
              <th>Notes</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(rec, idx) in game.recommendations" :key="idx">
              <td class="col-players">{{ rec.players }}</td>
              <td class="col-ram">{{ rec.ram_gb }} GB</td>
              <td class="col-cpu">{{ rec.cpu_cores }} cores</td>
              <td class="col-notes">{{ rec.notes }}</td>
            </tr>
          </tbody>
        </table>
      </details>
    </div>

    <div class="guide-footer">
      <p><strong>💡 Conseil:</strong> Les serveurs gourmands (ARK, Palworld) gagnent à avoir des CPU rapides (3.5+ GHz).</p>
      <p><strong>📌 Factorio:</strong> La taille de l'usine compte plus que le nombre de joueurs.</p>
      <p><strong>⏱️ Maintenance:</strong> Les serveurs très chargés bénéficient de redémarrages réguliers (12-24h).</p>
    </div>
  </div>
</template>

<style scoped>
.resources-guide {
  background: var(--color-background-muted);
  border-radius: 8px;
  padding: 24px;
  margin-bottom: 32px;
}

.guide-header {
  margin-bottom: 24px;
  border-bottom: 2px solid var(--color-border);
  padding-bottom: 16px;
}

.guide-header h3 {
  margin: 0 0 8px 0;
  font-size: 1.3em;
  color: var(--color-text-primary);
}

.guide-intro {
  margin: 0;
  color: var(--color-text-secondary);
  font-size: 0.95em;
}

.games-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 24px;
}

.game-section {
  border: 1px solid var(--color-border);
  border-radius: 6px;
  overflow: hidden;
  background: var(--color-background);
}

.game-section[open] {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.game-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  cursor: pointer;
  user-select: none;
  font-weight: 600;
  color: var(--color-text-primary);
  background: var(--color-background-muted);
  transition: background 0.2s;
}

.game-header:hover {
  background: var(--color-background-hover);
}

.game-section[open] > .game-header {
  border-bottom: 1px solid var(--color-border);
}

.game-icon {
  font-size: 1.4em;
  min-width: 24px;
}

.game-name {
  flex: 1;
}

.recommendations-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.95em;
}

.recommendations-table thead {
  background: var(--color-background-muted);
}

.recommendations-table th {
  padding: 10px 12px;
  text-align: left;
  font-weight: 600;
  color: var(--color-text-secondary);
  border-bottom: 1px solid var(--color-border);
}

.recommendations-table td {
  padding: 10px 12px;
  border-bottom: 1px solid var(--color-border-subtle);
}

.recommendations-table tbody tr:last-child td {
  border-bottom: none;
}

.recommendations-table tbody tr:hover {
  background: var(--color-background-hover);
}

.col-players,
.col-ram,
.col-cpu {
  font-weight: 500;
  font-family: monospace;
}

.col-notes {
  color: var(--color-text-secondary);
  font-size: 0.9em;
  font-style: italic;
}

.guide-footer {
  border-top: 2px solid var(--color-border);
  padding-top: 16px;
}

.guide-footer p {
  margin: 8px 0;
  color: var(--color-text-secondary);
  font-size: 0.9em;
}

.guide-footer strong {
  color: var(--color-text-primary);
}

@media (max-width: 768px) {
  .resources-guide {
    padding: 16px;
  }

  .recommendations-table {
    font-size: 0.85em;
  }

  .recommendations-table th,
  .recommendations-table td {
    padding: 8px;
  }
}
</style>
