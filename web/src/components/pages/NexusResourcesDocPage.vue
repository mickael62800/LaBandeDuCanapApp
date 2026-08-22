<script setup lang="ts">
// Documentation de dimensionnement des serveurs de jeu.
//
// Les chiffres ne sont PAS ecrits ici : ils viennent de `@/data/gameResources`,
// que le tableau de la page de creation lit aussi. Cette page ajoute ce qu'un
// tableau ne sait pas dire — les facteurs qui font reellement monter la
// consommation, et quoi faire quand le serveur ralentit.

import { ref } from "vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import GameResourcesGuide from "../organisms/GameResourcesGuide.vue";
import { gameResources } from "@/data/gameResources";

type Onglet = "tableaux" | "jeux" | "conseils";
const onglet = ref<Onglet>("tableaux");

const onglets: { cle: Onglet; libelle: string }[] = [
  { cle: "tableaux", libelle: "Tableaux" },
  { cle: "jeux", libelle: "Par jeu" },
  { cle: "conseils", libelle: "Conseils" },
];
</script>

<template>
  <AdminPageShell
    title="Ressources des serveurs de jeu"
    subtitle="Combien de mémoire et de cœurs pour combien de joueurs"
  >
    <nav class="rd-tabs" role="tablist">
      <button
        v-for="t in onglets"
        :key="t.cle"
        type="button"
        role="tab"
        :aria-selected="onglet === t.cle"
        :class="['rd-tab', { active: onglet === t.cle }]"
        @click="onglet = t.cle"
      >
        {{ t.libelle }}
      </button>
    </nav>

    <!-- Tableaux : le catalogue complet, sans slug -->
    <section v-if="onglet === 'tableaux'" class="rd-pane">
      <GameResourcesGuide />
    </section>

    <!-- Par jeu : ce que le tableau ne dit pas -->
    <section v-else-if="onglet === 'jeux'" class="rd-pane">
      <p class="rd-lede">
        Le tableau donne un point de départ. Ce qui décide vraiment de la
        consommation, ce sont ces facteurs — les lire évite de doubler la RAM
        quand le problème était ailleurs.
      </p>

      <article v-for="jeu in gameResources" :key="jeu.slug" class="rd-game">
        <h3 class="rd-game-title">
          <span aria-hidden="true">{{ jeu.icon }}</span> {{ jeu.name }}
        </h3>
        <ul class="rd-factors">
          <li v-for="(f, i) in jeu.facteurs" :key="i">{{ f }}</li>
        </ul>
        <p class="rd-range">
          De <strong>{{ jeu.recommendations[0]?.ram_gb }} Go</strong>
          ({{ jeu.recommendations[0]?.players }} joueurs)
          à
          <strong>{{ jeu.recommendations[jeu.recommendations.length - 1]?.ram_gb }} Go</strong>
          ({{ jeu.recommendations[jeu.recommendations.length - 1]?.players }} joueurs).
        </p>
      </article>
    </section>

    <!-- Conseils -->
    <section v-else class="rd-pane">
      <article class="rd-tip">
        <h3>Lire la surveillance avant d'ajouter de la mémoire</h3>
        <p>
          L'onglet « Surveillance » d'un serveur montre la mémoire et le
          processeur en direct. Il dit lequel des deux manque — doubler la RAM
          d'un serveur limité par le processeur ne change rien.
        </p>
        <ul>
          <li>Mémoire au plafond : augmenter l'allocation, ou réduire mods et distance de vue.</li>
          <li>Processeur au plafond : réduire le monde ou les joueurs ; les cœurs en plus aident peu.</li>
          <li>Latence élevée alors que les deux respirent : c'est le réseau.</li>
        </ul>
      </article>

      <article class="rd-tip">
        <h3>Redémarrer plutôt que sur-dimensionner</h3>
        <p>
          Plusieurs de ces jeux dérivent en mémoire tant qu'ils tournent :
          la consommation monte avec la durée, pas avec la charge.
        </p>
        <ul>
          <li>Palworld, ARK, 7 Days to Die : redémarrage quotidien.</li>
          <li>Minecraft, Valheim : une à deux fois par semaine suffit.</li>
          <li>Factorio : selon la taille de l'usine.</li>
        </ul>
      </article>

      <article class="rd-tip">
        <h3>Fréquence plutôt que vCPU alloués</h3>
        <p>
          Minecraft, Valheim, Factorio, Satisfactory, V Rising et Necesse font
          tourner leur boucle principale sur un seul thread. Ce que les guides
          d'hébergeurs appellent « 4 cœurs recommandés » encode en réalité une
          exigence de <strong>fréquence</strong> : un hôte à 4 GHz y bat un hôte
          à 2,4 GHz, quel que soit le quota accordé au conteneur.
        </p>
      </article>

      <article class="rd-tip">
        <h3>Ce que Docker fait de ces valeurs</h3>
        <p>
          La mémoire allouée est une limite <strong>stricte</strong> : le
          conteneur est tué s'il la dépasse. Le réglage processeur, lui, part en
          <code>--cpus</code> : un plafond de temps processeur, pas une
          réservation, et compté en processeurs <strong>logiques</strong>.
        </p>
        <ul>
          <li>
            Ce sont des <strong>threads</strong>, pas des cœurs physiques : avec
            Hyper-Threading, 4 vCPU valent environ 2 cœurs.
          </li>
          <li>
            En allouer plus n'accélère pas un moteur mono-thread — cela prive
            seulement les autres serveurs de l'hôte.
          </li>
          <li>Laisser de la marge à la machine hôte : au moins 2 Go hors serveurs.</li>
          <li>
            Docker fige ces limites à la création du conteneur : les modifier
            ne prend effet qu'au redémarrage du serveur.
          </li>
        </ul>
      </article>

      <article class="rd-tip">
        <h3>Les mods changent tout</h3>
        <p>
          Les tableaux décrivent des serveurs vanilla sauf mention contraire.
          Une installation moddée demande couramment 2 à 4 Go de plus, et sur
          Project Zomboid l'écart est encore plus net.
        </p>
      </article>
    </section>
  </AdminPageShell>
</template>

<style scoped>
.rd-tabs {
  display: flex;
  gap: var(--space-xs);
  border-bottom: 1px solid var(--border);
  margin-bottom: var(--space-lg);
  overflow-x: auto;
}

.rd-tab {
  padding: var(--space-sm) var(--space-md);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--text-secondary);
  font-weight: 500;
  font-size: 0.9rem;
  cursor: pointer;
  white-space: nowrap;
  transition: color var(--transition-fast), border-color var(--transition-fast);
}

.rd-tab:hover {
  color: var(--text-primary);
}

.rd-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.rd-lede {
  margin: 0 0 var(--space-lg);
  color: var(--text-secondary);
  font-size: 0.9rem;
  line-height: 1.6;
  max-width: 70ch;
}

.rd-game,
.rd-tip {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-md) var(--space-lg);
  margin-bottom: var(--space-md);
}

.rd-game-title,
.rd-tip h3 {
  margin: 0 0 var(--space-sm);
  font-size: 1rem;
  color: var(--text-primary);
}

.rd-factors,
.rd-tip ul {
  margin: 0;
  padding-left: 1.2rem;
  color: var(--text-secondary);
  font-size: 0.88rem;
  line-height: 1.6;
}

.rd-factors li,
.rd-tip li {
  margin-bottom: 0.2rem;
}

.rd-range {
  margin: var(--space-sm) 0 0;
  padding-top: var(--space-sm);
  border-top: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 0.85rem;
}

.rd-range strong {
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

.rd-tip p {
  margin: 0 0 var(--space-sm);
  color: var(--text-secondary);
  font-size: 0.88rem;
  line-height: 1.6;
  max-width: 70ch;
}

.rd-tip p:last-child {
  margin-bottom: 0;
}

.rd-tip strong {
  color: var(--text-primary);
}
</style>
