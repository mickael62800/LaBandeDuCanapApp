<script setup lang="ts">
// Les jeux de la communauté, jouables depuis le site.
//
// # Un seul portefeuille
//
// Rien n'est calculé ici. Le tirage, le quota quotidien et les mouvements de
// coins vivent dans nexus-core, appelé par le même chemin que le bot Discord.
// Le solde affiché EST celui de Discord — pas une copie synchronisée, le même.
// Avoir déjà tiré sur Discord fait échouer le tirage ici, et réciproquement.
//
// # L'animation
//
// La roue tourne pendant que la requête part, puis s'arrête sur la case que
// le serveur a réellement tirée. L'ordre compte : faire tourner puis demander
// le résultat laisserait croire que l'animation le détermine. Ici elle ne fait
// que le mettre en scène.

import { computed, onMounted, ref, watch } from "vue";

import ActionButton from "@/components/atoms/ActionButton.vue";
import SiteHero from "@/components/molecules/SiteHero.vue";
import CoussinProfilePanel from "@/components/organisms/games/CoussinProfilePanel.vue";
import GamesCatalogNavigation from "@/components/organisms/games/GamesCatalogNavigation.vue";
import GamesLeaderboardPanel from "@/components/organisms/games/GamesLeaderboardPanel.vue";
import GamesTransactionHistory from "@/components/organisms/games/GamesTransactionHistory.vue";
import WheelGamePanel from "@/components/organisms/games/WheelGamePanel.vue";
import { useAuth } from "@/composables/useAuth";
import {
  gamesService,
  type Rank,
  type SpinResult,
  type Transaction,
  type CoussinFile,
  type Wallet,
} from "@/services/gamesService";
import { GAMES, jeuMemorise, memoriserJeu } from "@/games/catalog";

const { user } = useAuth();

// ── Carrousel ──

/// Jeu affiché. Restauré du dernier passage : revenir sur la page doit
/// ramener là où on était, pas au début du catalogue.
const jeuActif = ref(jeuMemorise());

const jeu = computed(() => GAMES.find((g) => g.key === jeuActif.value) ?? GAMES[0]);
const indexActif = computed(() => GAMES.findIndex((g) => g.key === jeuActif.value));

function choisir(key: string) {
  jeuActif.value = key;
  memoriserJeu(key);
}

/// Décale d'un cran, en bouclant. Boucler plutôt que buter : avec deux ou
/// trois jeux, un bouton désactivé aux extrémités serait plus gênant qu'utile.
function decaler(pas: number) {
  const n = GAMES.length;
  choisir(GAMES[(indexActif.value + pas + n) % n].key);
}

const wallet = ref<Wallet | null>(null);
const history = ref<Transaction[]>([]);
const ranking = ref<Rank[]>([]);
const chargement = ref(true);
const indisponible = ref(false);

// ── Roue ──

/// Habillage des cases HISTORIQUES : emoji et couleur, que le serveur ne
/// stocke pas. Un serveur qui ajoute ses propres cases retombe sur un
/// habillage déduit du gain (voir `habillage`).
const HABILLAGE = [
  { key: "blanche", court: "Rien", emoji: "🌀", couleur: "#6b7280" },
  { key: "pq", court: "+50", emoji: "🧻", couleur: "#94a3b8" },
  { key: "sieste", court: "+200", emoji: "💤", couleur: "#38bdf8" },
  { key: "colis", court: "+500", emoji: "📦", couleur: "#22c55e" },
  { key: "trefle", court: "+1000", emoji: "🍀", couleur: "#16a34a" },
  { key: "couronne", court: "+1500", emoji: "👑", couleur: "#f39c12" },
  { key: "ruine", court: "-500", emoji: "💀", couleur: "#f43f5e" },
  { key: "jackpot", court: "+5000", emoji: "🎰", couleur: "#a855f7" },
  { key: "bombe", court: "-2000", emoji: "💣", couleur: "#dc2626" },
  { key: "licorne", court: "+10000", emoji: "🦄", couleur: "#e879f9" },
];

/// Les cases réellement en jeu, telles que le serveur les définit. Tant
/// qu'elles ne sont pas chargées, on dessine celles d'origine : une roue vide
/// pendant une seconde serait pire qu'une roue approximative.
const casesServeur = ref<{ key: string; label: string; payout: number }[]>([]);

/// Habillage d'une case : celui d'origine si la clé est connue, sinon déduit
/// du gain. Vert quand ça rapporte, rouge quand ça coûte, gris quand c'est
/// blanc — c'est la seule information dont on soit sûr pour une case inventée
/// par un serveur.
function habillage(key: string, payout: number) {
  const connu = HABILLAGE.find((c) => c.key === key);
  if (connu) return connu;
  const court = payout > 0 ? `+${payout}` : payout < 0 ? `${payout}` : "Rien";
  if (payout > 0) return { key, court, emoji: "🎁", couleur: "#22c55e" };
  if (payout < 0) return { key, court, emoji: "💥", couleur: "#f43f5e" };
  return { key, court, emoji: "🌀", couleur: "#6b7280" };
}

const CASES = computed(() =>
  (casesServeur.value.length
    ? casesServeur.value
    : HABILLAGE.map((h) => ({ key: h.key, label: h.court, payout: 0 }))
  ).map((c) => habillage(c.key, c.payout)),
);

/// L'angle d'un secteur dépend du NOMBRE de cases : une roue de six cases se
/// découpe en soixante degrés, pas en trente-six.
const SECTEUR = computed(() => 360 / Math.max(1, CASES.value.length));

const enCours = ref(false);
const resultat = ref<SpinResult | null>(null);
const erreurRoue = ref<string | null>(null);
/// Passe à vrai dès que le serveur refuse pour cause de tirage déjà consommé.
/// Sans ça, le bouton reste engageant et on réessaie indéfiniment un geste
/// dont le refus est certain.
const dejaJoue = ref(false);
/// Angle cumulé, jamais remis à zéro : revenir en arrière ferait tourner la
/// roue à l'envers entre deux tirages.
///
/// Départ à un demi-secteur, et non à zéro : à zéro la flèche tombe pile sur
/// la FRONTIÈRE entre la dernière case et la première. Elle semblait alors
/// désigner la licorne à chaque position de repos — y compris après un tirage
/// refusé, qui ramène la roue à son point de départ. Le hasard n'y était pour
/// rien, seule la géométrie.
const angle = ref(-360 / 20);

async function tirer() {
  if (enCours.value || dejaJoue.value || !user.value) return;
  enCours.value = true;
  erreurRoue.value = null;
  resultat.value = null;

  // Mémorisé pour pouvoir revenir ici si le tirage est refusé.
  const avant = angle.value;

  // Quelques tours pleins avant même de connaître l'issue : l'attente fait
  // partie du jeu, et la requête se déroule pendant ce temps.
  angle.value += 360 * 4;

  try {
    const r = await gamesService.spinWheel();

    const index = Math.max(0, CASES.value.findIndex((c) => c.key === r.case_key));
    // On complète jusqu'au secteur voulu, en restant dans le même sens.
    const secteur = SECTEUR.value;
    const vise = 360 - index * secteur - secteur / 2;
    const restant = (vise - (angle.value % 360) + 360) % 360;
    angle.value += restant;

    // Laisse l'animation finir avant d'annoncer : lire le gain pendant que la
    // roue tourne encore gâche le seul moment de suspense du jeu.
    await new Promise((r) => setTimeout(r, 3200));

    resultat.value = r;
    if (wallet.value) wallet.value.coins = r.balance_after;
    // Le tirage vient de créer une transaction : on recharge plutôt que de
    // la fabriquer côté client, où elle divergerait du libellé serveur.
    history.value = await gamesService.history();
    ranking.value = await gamesService.leaderboard();
  } catch (e) {
    const message = e instanceof Error ? e.message : "Le tirage a échoué.";
    erreurRoue.value = message;

    // Refus pour tirage déjà consommé : c'est définitif jusqu'à demain, on
    // ferme le bouton. Les autres échecs (réseau, plateforme éteinte) sont
    // passagers et méritent une nouvelle tentative.
    if (/déjà tiré|deja tire/i.test(message)) {
      dejaJoue.value = true;
    }

    // La roue revient exactement où elle était : la laisser finir ses trois
    // secondes ferait croire à un tirage qui n'a pas eu lieu.
    angle.value = avant;
  } finally {
    enCours.value = false;
  }
}

// ── Coussin Piégé ──

const coussin = ref<CoussinFile | null>(null);
const coussinErreur = ref<string | null>(null);

/// Chargé à la demande, pas au montage : quelqu'un qui vient pour la Roue
/// n'a pas à payer quatre requêtes pour un jeu qu'il ne regardera pas.
async function chargerCoussin() {
  if (coussin.value || !user.value) return;
  coussinErreur.value = null;
  try {
    coussin.value = await gamesService.coussin();
  } catch (e) {
    coussinErreur.value =
      e instanceof Error ? e.message : "Impossible de charger ton profil.";
  }
}

watch(
  () => jeuActif.value,
  (k) => {
    if (k === "coussin") void chargerCoussin();
  },
  { immediate: true },
);

// ── Chargement ──

onMounted(async () => {
  // Tentative silencieuse : l'endpoint demande d'être connecté, et un
  // visiteur anonyme gardera donc les cases d'origine à l'écran. C'est le bon
  // repli — une roue non dessinée serait pire qu'une roue approximative.
  gamesService
    .wheelCases()
    .then((r) => {
      casesServeur.value = r.cases;
    })
    .catch(() => {});

  if (!user.value) {
    chargement.value = false;
    return;
  }
  try {
    const [w, h, l] = await Promise.all([
      gamesService.wallet(),
      gamesService.history(),
      gamesService.leaderboard(),
    ]);
    wallet.value = w;
    history.value = h;
    ranking.value = l;
    // Le serveur sait déjà si le tirage du jour est consommé : on ferme le
    // bouton avant tout clic, au lieu de le faire découvrir par un refus.
    dejaJoue.value = !w.can_spin;
  } catch {
    // Plateforme jeux éteinte ou non configurée : on le dit, plutôt que
    // d'afficher un portefeuille vide qui ferait croire à une perte de coins.
    indisponible.value = true;
  } finally {
    chargement.value = false;
  }
});

// ── Affichage ──

const solde = computed(() => wallet.value?.coins ?? 0);

function fmtCoins(n: number): string {
  return n.toLocaleString("fr-FR");
}

/// Dégradé conique dessinant les secteurs. Recalculé seulement quand les
/// cases changent : le recalculer à chaque rendu ferait clignoter la roue
/// pendant sa rotation.
const fondRoue = computed(() => {
  const parts = CASES.value.map((c, i) => {
    const de = i * SECTEUR.value;
    const a = (i + 1) * SECTEUR.value;
    return `${c.couleur} ${de}deg ${a}deg`;
  });
  return `conic-gradient(${parts.join(", ")})`;
});
</script>

<template>
  <div class="jx theme-communaute">
    <!-- Le lien de retour vers l'espace membre est parti dans `SiteHeader`,
         qui porte la navigation des trois pages publiques. Ne reste ici que
         le solde : lui est propre a cette page. -->
    <div v-if="user" class="jx-bar">
      <span class="jx-solde" :class="{ pulse: !!resultat }">
        🪙 {{ fmtCoins(solde) }}
      </span>
    </div>

    <SiteHero
      taille="compact"
      titre="Les jeux du canapé"
      tagline="Le même porte-monnaie que sur Discord. Ce que tu gagnes ici, tu le retrouves là-bas."
    />

    <!-- Non connecté : on montre le jeu, on demande la connexion pour agir. -->
    <section v-if="!user" class="jx-block">
      <p class="jx-vide">
        Connecte-toi pour tirer la Roue et retrouver ton porte-monnaie.
      </p>
      <ActionButton to="/login?espace=membre">Se connecter</ActionButton>
    </section>

    <p v-else-if="chargement" class="jx-hint">Chargement…</p>

    <section v-else-if="indisponible" class="jx-block">
      <p class="jx-alerte">
        La plateforme de jeux ne répond pas. Ton porte-monnaie n'est pas perdu,
        il est simplement inaccessible pour l'instant.
      </p>
    </section>

    <template v-else>
      <GamesCatalogNavigation
        :games="GAMES"
        :active-key="jeuActif"
        @select="choisir"
        @shift="decaler"
      />

      <WheelGamePanel
        v-if="jeuActif === 'roue'"
        :cases="CASES"
        :sector="SECTEUR"
        :background="fondRoue"
        :angle="angle"
        :spinning="enCours"
        :already-played="dejaJoue"
        :error="erreurRoue"
        :result="resultat"
        @spin="tirer"
      />

      <CoussinProfilePanel
        v-else-if="jeuActif === 'coussin'"
        :game="jeu"
        :file="coussin"
        :error="coussinErreur"
      />

      <section v-else class="jx-block">
        <h2>{{ jeu.emoji }} {{ jeu.nom }}</h2>
        <p class="jx-vide">{{ jeu.pitch }}</p>
      </section>

      <GamesLeaderboardPanel :ranking="ranking" />
      <GamesTransactionHistory :transactions="history" />
    </template>
  </div>
</template>

<style src="../../../styles/public-games.css"></style>
