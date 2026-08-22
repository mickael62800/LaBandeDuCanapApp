<script setup lang="ts">
import AdminPageShell from "../layouts/AdminPageShell.vue";
import GameResourcesGuide from "../organisms/GameResourcesGuide.vue";
import { ref } from "vue";

const activeTab = ref<"overview" | "detailed" | "tips">("overview");
</script>

<template>
  <AdminPageShell title="📚 Ressources & Hébergement Nexus" subtitle="Guide complet pour dimensionner tes serveurs de jeu">
    <div class="doc-container">
      <!-- Tabs de navigation -->
      <div class="doc-tabs">
        <button
          :class="{ active: activeTab === 'overview' }"
          @click="activeTab = 'overview'"
        >
          Vue d'ensemble
        </button>
        <button
          :class="{ active: activeTab === 'detailed' }"
          @click="activeTab = 'detailed'"
        >
          Guides détaillés
        </button>
        <button
          :class="{ active: activeTab === 'tips' }"
          @click="activeTab = 'tips'"
        >
          Conseils & Optimisation
        </button>
      </div>

      <!-- TAB 1: Vue d'ensemble -->
      <section v-if="activeTab === 'overview'" class="doc-section">
        <GameResourcesGuide />

        <div class="info-box">
          <h4>🎯 Comment lire ce guide ?</h4>
          <ul>
            <li><strong>Joueurs :</strong> Nombre de joueurs simultanés.</li>
            <li><strong>RAM :</strong> Mémoire vive requise en GB. Les conteneurs Docker respectent cette limite strictement.</li>
            <li><strong>CPU :</strong> Nombre de cœurs (cores) recommandés. La fréquence (GHz) compte souvent autant que le nombre.</li>
            <li><strong>Notes :</strong> Contexte de la configuration (vanilla, mods, exploration, etc.).</li>
          </ul>
        </div>

        <div class="summary-grid">
          <div class="summary-card">
            <h4>💾 RAM: Règle générale</h4>
            <p>Commence bas et augmente si le serveur ralentit ou crash (out of memory).</p>
          </div>
          <div class="summary-card">
            <h4>⚡ CPU: Règle générale</h4>
            <p>La fréquence (3+ GHz) vaut souvent mieux que la quantité de cœurs.</p>
          </div>
          <div class="summary-card">
            <h4>🔄 Ajustement</h4>
            <p>Vérifie les logs du serveur et redémarre le régulièrement (12-24h).</p>
          </div>
          <div class="summary-card">
            <h4>🌍 Monde / Usine</h4>
            <p>Pour Minecraft/Factorio/ARK, c'est la taille/complexité qui compte plus que les joueurs.</p>
          </div>
        </div>
      </section>

      <!-- TAB 2: Guides détaillés par jeu -->
      <section v-if="activeTab === 'detailed'" class="doc-section">
        <div class="game-detail">
          <h3>⛏️ Minecraft Java</h3>
          <p class="game-desc">Serveur vanilla ou avec plugins/mods. Très flexible en ressources.</p>
          <div class="detail-content">
            <div class="detail-col">
              <h5>Facteurs clés</h5>
              <ul>
                <li>Distance de vue (view distance)</li>
                <li>Nombre de plugins</li>
                <li>Génération du monde</li>
                <li>Nombre de joueurs éloignés</li>
              </ul>
            </div>
            <div class="detail-col">
              <h5>Recommandations</h5>
              <ul>
                <li><strong>4 joueurs :</strong> 4 GB RAM, 2 CPU</li>
                <li><strong>10 joueurs :</strong> 6-8 GB RAM, 4 CPU</li>
                <li><strong>20 joueurs :</strong> 8-12 GB RAM, 4 CPU (plugins lourds)</li>
              </ul>
            </div>
          </div>
        </div>

        <div class="game-detail">
          <h3>🪓 Valheim</h3>
          <p class="game-desc">Survie coopérative vikings, limitée à 10 joueurs max. Très stable.</p>
          <div class="detail-content">
            <div class="detail-col">
              <h5>Facteurs clés</h5>
              <ul>
                <li>Taille du monde (exploré)</li>
                <li>Nombre de constructions</li>
                <li>Nombre de joueurs max: <strong>10</strong></li>
              </ul>
            </div>
            <div class="detail-col">
              <h5>Recommandations</h5>
              <ul>
                <li><strong>5 joueurs :</strong> 4 GB RAM, 2 CPU (3+ GHz)</li>
                <li><strong>10 joueurs :</strong> 6-8 GB RAM, 4 CPU</li>
              </ul>
            </div>
          </div>
        </div>

        <div class="game-detail">
          <h3>⚙️ Factorio</h3>
          <p class="game-desc">Logistique industrielle. Scalabilité: la taille de l'usine > nombre de joueurs.</p>
          <div class="detail-content">
            <div class="detail-col">
              <h5>Facteurs clés</h5>
              <ul>
                <li><strong>Taille de l'usine</strong> (plus important)</li>
                <li>Nombre de mods</li>
                <li>Uptime sans redémarrage</li>
              </ul>
            </div>
            <div class="detail-col">
              <h5>Recommandations</h5>
              <ul>
                <li><strong>Petite usine (4 joueurs) :</strong> 3 GB, 2 CPU</li>
                <li><strong>Usine moyenne (8 joueurs) :</strong> 4-6 GB, 4 CPU</li>
                <li><strong>Grosse usine :</strong> 6-8 GB, 4 CPU</li>
              </ul>
            </div>
          </div>
        </div>

        <div class="game-detail">
          <h3>🐾 Palworld</h3>
          <p class="game-desc">Survie avec créatures. Très gourmand en RAM, surtout avec des bases actives.</p>
          <div class="detail-content">
            <div class="detail-col">
              <h5>Facteurs clés</h5>
              <ul>
                <li>Nombre de bases</li>
                <li>Activité des Pals (simulation continue)</li>
                <li>Uptime entre redémarrages</li>
              </ul>
            </div>
            <div class="detail-col">
              <h5>Recommandations</h5>
              <ul>
                <li><strong>8 joueurs :</strong> 8 GB (redémarrage quotidien)</li>
                <li><strong>16 joueurs :</strong> 16 GB (recommandation officielle)</li>
                <li><strong>32 joueurs :</strong> 24+ GB</li>
              </ul>
            </div>
          </div>
        </div>

        <div class="game-detail">
          <h3>🦖 ARK: Survival Evolved</h3>
          <p class="game-desc">Dinosaures et exploration. RAM critique; mods = RAM supplémentaire.</p>
          <div class="detail-content">
            <div class="detail-col">
              <h5>Facteurs clés</h5>
              <ul>
                <li>Carte (Théisland > Extinction)</li>
                <li>Nombre de mods</li>
                <li>Créatures apprivoisées</li>
              </ul>
            </div>
            <div class="detail-col">
              <h5>Recommandations</h5>
              <ul>
                <li><strong>10 joueurs :</strong> 8 GB, 2 CPU</li>
                <li><strong>15 joueurs :</strong> 12-16 GB, 4 CPU</li>
                <li><strong>20+ joueurs :</strong> 16-20 GB, 4 CPU</li>
              </ul>
            </div>
          </div>
        </div>

        <div class="game-detail">
          <h3>🧟 7 Days to Die</h3>
          <p class="game-desc">Zombies. CPU rapide + RAM stable; redémarrages réguliers recommandés.</p>
          <div class="detail-content">
            <div class="detail-col">
              <h5>Facteurs clés</h5>
              <ul>
                <li>CPU rapide (3+ GHz) critique</li>
                <li>Mods (beaucoup ralentissent)</li>
                <li>Taille monde exploré</li>
              </ul>
            </div>
            <div class="detail-col">
              <h5>Recommandations</h5>
              <ul>
                <li><strong>4 joueurs (vanilla) :</strong> 4-6 GB, 4 CPU</li>
                <li><strong>8 joueurs (modded) :</strong> 12-16 GB, 6 CPU (4+ GHz)</li>
              </ul>
            </div>
          </div>
        </div>

        <div class="game-detail">
          <h3>🌳 Terraria</h3>
          <p class="game-desc">Bac à sable 2D léger. Demande peu de ressources; CPU compte lors de boss fights.</p>
          <div class="detail-content">
            <div class="detail-col">
              <h5>Facteurs clés</h5>
              <ul>
                <li>Très peu gourmand</li>
                <li>Mods changent la donne</li>
                <li>Boss fights = pics CPU</li>
              </ul>
            </div>
            <div class="detail-col">
              <h5>Recommandations</h5>
              <ul>
                <li><strong>3 joueurs :</strong> 512 MB - 1 GB, 1-2 CPU</li>
                <li><strong>10 joueurs :</strong> 1-2 GB, 2-4 CPU</li>
                <li><strong>Modded :</strong> 2-4 GB, 4 CPU</li>
              </ul>
            </div>
          </div>
        </div>
      </section>

      <!-- TAB 3: Conseils & Optimisation -->
      <section v-if="activeTab === 'tips'" class="doc-section">
        <div class="tips-group">
          <h3>🔧 Optimisation & Maintenance</h3>

          <div class="tip-card">
            <h4>📊 Surveiller ton serveur</h4>
            <p>Utilise l'onglet <strong>« Surveillance »</strong> pour vérifier RAM/CPU/latence en temps réel.</p>
            <ul>
              <li>RAM > 85% ? Augmente l'allocation ou réduis les joueurs/mods.</li>
              <li>CPU > 90% ? Relève la fréquence (redémarrage) ou réduis le monde.</li>
              <li>Latence ping > 300ms ? Problème réseau ou CPU saturé.</li>
            </ul>
          </div>

          <div class="tip-card">
            <h4>🔄 Redémarrages réguliers</h4>
            <p>Les serveurs lourds bénéficient de redémarrages quotidiens (12-24h).</p>
            <ul>
              <li><strong>Palworld, ARK, 7DTD :</strong> Redémarrage quotidien recommandé.</li>
              <li><strong>Minecraft, Valheim :</strong> 1-2x par semaine suffisent.</li>
              <li><strong>Factorio :</strong> Selon la taille de l'usine, 2-3x/semaine.</li>
            </ul>
          </div>

          <div class="tip-card">
            <h4>⚙️ Configuration Docker</h4>
            <p>
              Nexus gère automatiquement les limits RAM/CPU Docker.
              Tes valeurs sont respectées à la lettre (hard limits OOM-kill).
            </p>
            <ul>
              <li>Ne mets jamais plus de RAM que la machine n'en dispose.</li>
              <li>Laisse de la RAM libre pour le système (≥ 2 GB).</li>
              <li>CPU limit = nombre de cœurs; p.ex., 2.0 = 2 cores, 4.0 = 4 cores.</li>
            </ul>
          </div>

          <div class="tip-card">
            <h4>🌐 Fréquence CPU vs Nombre de cœurs</h4>
            <p>
              <strong>La plupart des jeux bénéficient plus d'une fréquence rapide que d'un nombre élevé de cœurs.</strong>
            </p>
            <ul>
              <li>Vise 3.5+ GHz pour les jeux exigeants (ARK, Palworld, 7DTD).</li>
              <li>Un 4-core à 4 GHz > 8-core à 2.4 GHz pour presque tous les jeux.</li>
              <li>Minecraft, Valheim, Factorio utilisent principalement un seul cœur pour la logique.</li>
            </ul>
          </div>

          <div class="tip-card">
            <h4>💾 Stockage</h4>
            <p>Nexus monte automatiquement des volumes Docker pour chaque serveur.</p>
            <ul>
              <li>Utilise du SSD si possible (bien plus rapide que HDD).</li>
              <li>Vérifie l'espace disque régulièrement (surtout Minecraft/ARK qui génèrent beaucoup de monde).</li>
            </ul>
          </div>

          <div class="tip-card">
            <h4>📈 Augmenter les ressources</h4>
            <p>Si ton serveur ralentit ou crash:</p>
            <ol>
              <li>Vérifie les logs (onglet « Logs ») pour le type d'erreur.</li>
              <li>Si « out of memory », augmente la RAM de 2-4 GB.</li>
              <li>Si processeur surchargé, augmente les CPU ou réduis les joueurs.</li>
              <li>Redémarre le serveur pour appliquer (arrête puis relance).</li>
            </ol>
          </div>

          <div class="tip-card">
            <h4>❌ Erreurs courantes</h4>
            <ul>
              <li><strong>« Out of Memory »:</strong> Allocation RAM insuffisante → augmente.</li>
              <li><strong>Slow IO :</strong> Disque trop lent ou saturé → redémarrage ou SSD.</li>
              <li><strong>High latency :</strong> Réseau ou CPU → vérifie surveillance.</li>
              <li><strong>Crash au démarrage :</strong> Config invalide → vérifie les logs.</li>
            </ul>
          </div>
        </div>

        <div class="tips-group">
          <h3>🎮 Par type de jeu</h3>

          <div class="tip-card">
            <h4>Jeux de survie (Minecraft, Valheim, ARK)</h4>
            <ul>
              <li>La taille du monde exploré augmente avec le temps → RAM croissante.</li>
              <li>Redémarrages réguliers rechargent le monde frais.</li>
              <li>Limite la distance de vue si RAM est limitée.</li>
            </ul>
          </div>

          <div class="tip-card">
            <h4>Jeux de simulation (Factorio)</h4>
            <ul>
              <li>Usine = plus important que joueurs. Grosse usine = besoin RAM/CPU élevé.</li>
              <li>Désactive les mods inutiles si performance baisse.</li>
              <li>Redémarrages quotidiens aident à stabiliser les UPS (updates/sec).</li>
            </ul>
          </div>

          <div class="tip-card">
            <h4>Jeux avec mods (7DTD, ARK, Factorio)</h4>
            <ul>
              <li>Les mods consomment toujours plus de ressources → prévois +2-4 GB.</li>
              <li>Test les mods en petit avant lancer au public.</li>
              <li>Les mods incompatibles causent souvent des crashes → logs!</li>
            </ul>
          </div>
        </div>
      </section>
    </div>
  </AdminPageShell>
</template>

<style scoped>
.doc-container {
  background: var(--color-background);
  border-radius: 8px;
  overflow: hidden;
}

.doc-tabs {
  display: flex;
  gap: 0;
  border-bottom: 2px solid var(--color-border);
  background: var(--color-background-muted);
  padding: 0 16px;
  overflow-x: auto;
}

.doc-tabs button {
  padding: 12px 20px;
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  border-bottom: 3px solid transparent;
  white-space: nowrap;
}

.doc-tabs button:hover {
  color: var(--color-text-primary);
  background: var(--color-background-hover);
}

.doc-tabs button.active {
  color: var(--color-accent);
  border-bottom-color: var(--color-accent);
}

.doc-section {
  padding: 24px;
  animation: fadeIn 0.2s;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.info-box {
  background: var(--color-background-muted);
  border-left: 4px solid var(--color-accent);
  padding: 16px;
  border-radius: 4px;
  margin: 24px 0;
}

.info-box h4 {
  margin: 0 0 12px 0;
  color: var(--color-accent);
}

.info-box ul {
  margin: 0;
  padding-left: 20px;
}

.info-box li {
  margin: 6px 0;
  color: var(--color-text-secondary);
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 16px;
  margin: 24px 0;
}

.summary-card {
  background: var(--color-background-muted);
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 16px;
  transition: all 0.2s;
}

.summary-card:hover {
  border-color: var(--color-accent);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.summary-card h4 {
  margin: 0 0 8px 0;
  color: var(--color-accent);
  font-size: 0.95em;
}

.summary-card p {
  margin: 0;
  font-size: 0.9em;
  color: var(--color-text-secondary);
}

.game-detail {
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 20px;
  margin-bottom: 16px;
  background: var(--color-background-muted);
}

.game-detail h3 {
  margin: 0 0 8px 0;
  color: var(--color-text-primary);
  font-size: 1.1em;
}

.game-desc {
  margin: 0 0 16px 0;
  color: var(--color-text-secondary);
  font-size: 0.95em;
  font-style: italic;
}

.detail-content {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
}

@media (max-width: 768px) {
  .detail-content {
    grid-template-columns: 1fr;
  }
}

.detail-col h5 {
  margin: 0 0 12px 0;
  color: var(--color-accent);
  font-size: 0.95em;
}

.detail-col ul {
  margin: 0;
  padding-left: 20px;
}

.detail-col li {
  margin: 6px 0;
  color: var(--color-text-secondary);
  font-size: 0.9em;
}

.tips-group {
  margin-bottom: 32px;
}

.tips-group h3 {
  color: var(--color-text-primary);
  margin-bottom: 16px;
  border-bottom: 2px solid var(--color-border);
  padding-bottom: 12px;
}

.tip-card {
  background: var(--color-background-muted);
  border-left: 4px solid var(--color-accent);
  padding: 16px;
  margin-bottom: 12px;
  border-radius: 4px;
}

.tip-card h4 {
  margin: 0 0 10px 0;
  color: var(--color-accent);
  font-size: 0.95em;
}

.tip-card p {
  margin: 0 0 10px 0;
  color: var(--color-text-secondary);
  font-size: 0.95em;
}

.tip-card p strong {
  color: var(--color-text-primary);
}

.tip-card ul,
.tip-card ol {
  margin: 0;
  padding-left: 20px;
  color: var(--color-text-secondary);
  font-size: 0.9em;
}

.tip-card li {
  margin: 6px 0;
}
</style>
