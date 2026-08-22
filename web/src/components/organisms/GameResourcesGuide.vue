<script setup lang="ts">
// Tableau indicatif « combien de RAM et de coeurs pour N joueurs ».
//
// Les chiffres vivent dans `@/data/gameResources` : la page de documentation
// les affiche aussi, et deux copies auraient diverge a la premiere correction.
//
// La page de creation passe le `slug` du jeu choisi : n'afficher que ce jeu
// evite de faire chercher sa ligne dans un tableau de quatorze entrees, au
// moment precis ou l'utilisateur regle la memoire. Sans `slug`, tout
// s'affiche — c'est le mode de la page de documentation.

import { computed } from "vue";
import { gameResources, trouverParSlug } from "@/data/gameResources";

const props = defineProps<{
  /// Slug du jeu choisi. Absent = catalogue complet.
  slug?: string;
}>();

/// Le jeu correspondant au slug demande, s'il est connu du guide.
const jeuChoisi = computed(() =>
  props.slug ? trouverParSlug(props.slug) : undefined,
);

/// Un slug inconnu (jeu ajoute au catalogue sans etre documente ici) ne doit
/// pas donner un panneau vide : on retombe sur le catalogue complet, en le
/// signalant, plutot que de laisser croire qu'aucune recommandation n'existe.
const slugInconnu = computed(() => Boolean(props.slug) && !jeuChoisi.value);

const jeuxAffiches = computed(() =>
  jeuChoisi.value ? [jeuChoisi.value] : gameResources,
);

/// Un seul jeu : le titre le nomme, et la section n'a plus a etre repliable.
const modeFiche = computed(() => Boolean(jeuChoisi.value));
</script>

<template>
  <div class="rg">
    <div class="rg-head">
      <h3 class="rg-title">
        💾 Recommandations de ressources<template v-if="jeuChoisi"> — {{ jeuChoisi.name }}</template>
      </h3>
      <p class="rg-intro">
        Valeurs <strong>indicatives</strong> pour dimensionner la mémoire et les cœurs.
        La consommation réelle dépend surtout de la taille du monde et des mods :
        commence bas, puis ajuste avec l'onglet « Surveillance ».
      </p>
      <p v-if="slugInconnu" class="rg-warn">
        Ce jeu n'a pas encore de fiche dédiée : voici le catalogue complet à titre de repère.
      </p>
    </div>

    <div class="rg-list">
      <component
        :is="modeFiche ? 'div' : 'details'"
        v-for="game in jeuxAffiches"
        :key="game.slug"
        class="rg-game"
        :open="modeFiche ? undefined : true"
      >
        <component
          :is="modeFiche ? 'div' : 'summary'"
          class="rg-game-head"
          :class="{ 'is-static': modeFiche }"
        >
          <span class="rg-game-icon" aria-hidden="true">{{ game.icon }}</span>
          <span class="rg-game-name">{{ game.name }}</span>
        </component>

        <div class="rg-table-wrap">
          <table class="rg-table">
            <thead>
              <tr>
                <th scope="col">Joueurs</th>
                <th scope="col">RAM</th>
                <th scope="col">Cœurs</th>
                <th scope="col">Notes</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="rec in game.recommendations" :key="rec.players">
                <td class="rg-num">{{ rec.players }}</td>
                <td class="rg-num">{{ rec.ram_gb }} Go</td>
                <td class="rg-num">{{ rec.cpu_cores }}</td>
                <td class="rg-notes">{{ rec.notes }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </component>
    </div>

    <ul class="rg-foot">
      <li>
        <strong>Fréquence &gt; nombre de cœurs.</strong>
        Un 4 cœurs à 4 GHz bat un 8 cœurs à 2,4 GHz sur presque tous ces jeux.
      </li>
      <li>
        <strong>Redémarrages.</strong>
        Palworld, ARK et 7 Days to Die dérivent en mémoire : un redémarrage quotidien
        vaut mieux que quelques Go de plus.
      </li>
      <li>
        <strong>Ce n'est pas le nombre de joueurs qui pèse le plus</strong>
        pour Factorio (taille de l'usine), V Rising (châteaux) et Necesse (colonies).
      </li>
    </ul>
  </div>
</template>

<style scoped>
.rg {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--space-lg);
  margin-bottom: var(--space-xl);
}

.rg-head {
  margin-bottom: var(--space-lg);
}

.rg-title {
  margin: 0 0 var(--space-xs);
  font-size: 1.05rem;
  color: var(--text-primary);
}

.rg-intro {
  margin: 0;
  color: var(--text-secondary);
  font-size: 0.9rem;
  line-height: 1.5;
}

.rg-warn {
  margin: var(--space-sm) 0 0;
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-sm);
  background: var(--warning-bg);
  color: var(--text-primary);
  font-size: 0.85rem;
}

.rg-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.rg-game {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--bg-primary);
}

.rg-game-head {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-md);
  font-weight: 600;
  color: var(--text-primary);
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  user-select: none;
}

.rg-game-head.is-static {
  cursor: default;
}

/* Sans cette regle, Chrome garde son triangle par defaut EN PLUS du notre. */
.rg-game-head::-webkit-details-marker {
  display: none;
}

.rg-game-icon {
  font-size: 1.2em;
}

.rg-game-name {
  flex: 1;
}

/* Un tableau ne doit jamais elargir la page : il defile dans son cadre. */
.rg-table-wrap {
  overflow-x: auto;
}

.rg-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.9rem;
}

.rg-table th {
  padding: var(--space-sm) var(--space-md);
  text-align: left;
  font-weight: 600;
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--text-secondary);
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-strong);
  white-space: nowrap;
}

.rg-table td {
  padding: var(--space-sm) var(--space-md);
  color: var(--text-primary);
  border-bottom: 1px solid var(--border);
}

.rg-table tbody tr:last-child td {
  border-bottom: none;
}

/* Zebrage : les lignes paires sont assombries pour que l'oeil suive une ligne
   entiere sans deriver sur la voisine — c'est un tableau large et court.
   Un voile noir plutot que `--muted-bg`, qui est un gris translucide : sur les
   fonds sombres du theme il ECLAIRCIT la ligne, l'inverse de l'effet voulu. */
.rg-table tbody tr:nth-child(even) {
  background: rgba(0, 0, 0, 0.22);
}

.rg-table tbody tr:hover {
  background: var(--bg-hover);
}

.rg-num {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
  white-space: nowrap;
}

.rg-notes {
  color: var(--text-secondary);
  font-size: 0.85rem;
}

.rg-foot {
  margin: var(--space-lg) 0 0;
  padding: var(--space-md) 0 0;
  border-top: 1px solid var(--border);
  list-style: none;
}

.rg-foot li {
  margin: 0 0 var(--space-xs);
  padding-left: var(--space-md);
  position: relative;
  color: var(--text-secondary);
  font-size: 0.85rem;
  line-height: 1.5;
}

.rg-foot li::before {
  content: "•";
  position: absolute;
  left: 0;
  color: var(--accent);
}

.rg-foot strong {
  color: var(--text-primary);
}

@media (max-width: 640px) {
  .rg {
    padding: var(--space-md);
  }

  .rg-table {
    font-size: 0.85rem;
  }
}
</style>
