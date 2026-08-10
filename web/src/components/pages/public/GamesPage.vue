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
import { useAuth } from "@/composables/useAuth";
import {
  gamesService,
  type Rank,
  type SpinResult,
  type Transaction,
  type CoussinCombat,
  type CoussinFile,
  type Wallet,
} from "@/services/gamesService";
import { badgeCanaux, GAMES, jeuMemorise, memoriserJeu } from "@/games/catalog";

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

/// Libellés des classes : le serveur renvoie une clé technique.
const CLASSES: Record<string, { nom: string; emoji: string; trait: string }> = {
  ecraseur: { nom: "Écraseur", emoji: "🪑", trait: "S'assoit sans regarder. Ça fait mal aux deux." },
  ressort: { nom: "Ressort", emoji: "🤸", trait: "Rebondit d'un accoudoir à l'autre." },
  piegeur: { nom: "Piégeur", emoji: "🪡", trait: "Place les coussins et fouille sous ceux des autres." },
  couette: { nom: "Couette", emoji: "🛌", trait: "Roulé dans la couette. Ne bougera plus." },
};

const classe = computed(() => {
  const k = coussin.value?.profile.class ?? "";
  return (
    CLASSES[k] ?? {
      nom: "Debout",
      emoji: "🧍",
      trait: "Tu n'as pas encore choisi ta place sur le canapé",
    }
  );
});

/// Part de victoires. `null` quand rien n'a été joué : afficher « 0 % » à
/// quelqu'un qui n'a jamais combattu serait un jugement, pas une statistique.
const tauxVictoire = computed(() => {
  const p = coussin.value?.profile;
  if (!p) return null;
  const total = p.total_wins + p.total_losses + p.total_draws;
  return total === 0 ? null : Math.round((p.total_wins / total) * 100);
});

/// Le combat a-t-il été gagné par le lecteur ?
function gagne(c: CoussinCombat): boolean | null {
  if (!c.winner_id) return null; // égalité
  return c.winner_id === adversaireContexte(c).moi;
}

/// Identifie le lecteur et son adversaire dans un combat, quel que soit le
/// côté où il se trouvait.
function adversaireContexte(c: CoussinCombat): { moi: string; nom: string } {
  const p = coussin.value?.profile;
  // Le profil ne porte pas l'identifiant : on se reconnaît au pseudo, qui
  // est celui enregistré au moment du combat.
  const jeSuisAttaquant = c.attacker_name === p?.username;
  return jeSuisAttaquant
    ? { moi: c.attacker_id, nom: c.defender_name }
    : { moi: c.defender_id, nom: c.attacker_name };
}

/// Objets : le serveur renvoie une clé technique, on l'habille.
/// Les clés du coffre à coussins, telles que le serveur les stocke. Elles
/// datent d'avant le changement de nom : les renommer viderait les
/// inventaires déjà constitués, donc seul l'affichage change.
const OBJETS: Record<string, string> = {
  rage: "🧱 Coussin Plombé",
  mindgame: "👁️ Œil sous le Plaid",
  explosion: "🍟 Renversement de Chips",
  double_coup: "🛋️ Double Coussin",
  surprise: "🪶 Bataille d'Oreillers",
  coup_traitre: "📌 Punaise dans le Coussin",
  inversion: "🔄 Retourne le Canapé",
};

function objet(cle: string): string {
  return OBJETS[cle] ?? `📦 ${cle}`;
}

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

function fmtDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/// Icône selon l'origine du mouvement. Le libellé technique du serveur ne
/// doit pas remonter tel quel à l'écran.
function icone(source: string): string {
  if (source.startsWith("wheel")) return "🎡";
  if (source.includes("transfer")) return "🤝";
  if (source.includes("coussin")) return "💥";
  return "🪙";
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
      <!-- ── Choix du jeu ── -->
      <section class="jx-block">
        <div class="jx-carrousel">
          <button
            v-if="GAMES.length > 1"
            type="button"
            class="jx-fleche-nav"
            aria-label="Jeu précédent"
            @click="decaler(-1)"
          >‹</button>

          <ul class="jx-vignettes">
            <li v-for="g in GAMES" :key="g.key">
              <button
                type="button"
                class="jx-vignette"
                :class="{ active: g.key === jeuActif }"
                :style="{ '--c': g.couleur }"
                @click="choisir(g.key)"
              >
                <span class="jx-vignette-emoji" aria-hidden="true">{{ g.emoji }}</span>
                <span class="jx-vignette-nom">{{ g.nom }}</span>
                <span class="jx-vignette-tag" :class="{ double: g.canaux.length > 1 }">
                  {{ badgeCanaux(g) }}
                </span>
              </button>
            </li>
          </ul>

          <button
            v-if="GAMES.length > 1"
            type="button"
            class="jx-fleche-nav"
            aria-label="Jeu suivant"
            @click="decaler(1)"
          >›</button>
        </div>

        <p class="jx-pitch">{{ jeu.pitch }}</p>
      </section>

      <!-- ── La Roue ── -->
      <section v-if="jeuActif === 'roue'" class="jx-block">
        <h2>La Roue du Destin <span class="jx-count">un tirage par jour</span></h2>

        <div class="jx-roue-zone">
          <div class="jx-roue-wrap">
            <span class="jx-fleche" aria-hidden="true"></span>
            <div
              class="jx-roue"
              :style="{
                background: fondRoue,
                transform: `rotate(${angle}deg)`,
              }"
            >
              <span
                v-for="(c, i) in CASES"
                :key="c.key"
                class="jx-case"
                :style="{ transform: `rotate(${i * SECTEUR + SECTEUR / 2}deg)` }"
              >
                <span class="jx-case-in">{{ c.emoji }}</span>
              </span>
            </div>
          </div>

          <div class="jx-roue-cote">
            <ActionButton
              size="lg"
              :disabled="enCours || dejaJoue"
              @click="tirer"
            >
              <template v-if="enCours">Ça tourne…</template>
              <template v-else-if="dejaJoue">Reviens demain</template>
              <template v-else>Tirer la Roue</template>
            </ActionButton>

            <p v-if="erreurRoue" :class="dejaJoue ? 'jx-vide' : 'jx-alerte'">
              {{ erreurRoue }}
            </p>

            <div v-else-if="resultat" class="jx-resultat" :class="{ rare: resultat.is_memorable }">
              <strong>{{ resultat.case_label }}</strong>
              <span
                v-if="resultat.payout !== 0"
                class="jx-gain"
                :class="resultat.payout > 0 ? 'plus' : 'moins'"
              >
                {{ resultat.payout > 0 ? "+" : "" }}{{ fmtCoins(resultat.payout) }} coins
              </span>
              <span v-else class="jx-gain neutre">Rien. Du tout.</span>
              <span class="jx-apres">Nouveau solde : {{ fmtCoins(resultat.balance_after) }}</span>
            </div>

            <p v-else class="jx-vide">
              Dix cases, de la ruine à la licorne. Le tirage est le même que
              celui de <code>/roue</code> sur Discord.
            </p>
          </div>
        </div>
      </section>

      <!-- Coussin Piégé : le jeu se JOUE sur Discord, mais tout ce qu'on y a
           accompli se consulte ici. Le web fait ce que Discord fait mal —
           garder une trace lisible. -->
      <section v-else-if="jeuActif === 'coussin'" class="jx-block">
        <h2>{{ jeu.emoji }} {{ jeu.nom }}</h2>

        <p v-if="!user" class="jx-vide">
          Connecte-toi pour voir ta place, tes bagarres et ce que tu planques.
        </p>
        <p v-else-if="coussinErreur" class="jx-alerte">{{ coussinErreur }}</p>
        <p v-else-if="!coussin" class="jx-hint">Chargement de ta fiche…</p>

        <template v-else>
          <!-- ── Fiche du personnage ── -->
          <div class="cd-fiche">
            <div class="cd-classe">
              <span class="cd-classe-emoji" aria-hidden="true">{{ classe.emoji }}</span>
              <div>
                <div class="cd-classe-nom">{{ classe.nom }}</div>
                <div class="cd-classe-trait">{{ classe.trait }}</div>
              </div>
            </div>

            <div class="cd-identite">
              <strong>{{ coussin.profile.username || "Toi" }}</strong>
              <span v-if="coussin.profile.title" class="cd-titre">
                « {{ coussin.profile.title }} »
              </span>
              <span class="cd-niveau">Niveau {{ coussin.profile.level }}</span>
            </div>

            <!-- Confort : une jauge dit d'un coup d'œil ce qu'un
                 « 34/50 » demande de calculer. À zéro on ne meurt pas, on se
                 lève du canapé — d'où le mot, et pas « points de vie ». -->
            <div class="cd-pv">
              <div class="cd-pv-ligne">
                <span>Confort</span>
                <span>{{ coussin.profile.hp_current }} / {{ coussin.profile.hp_max }}</span>
              </div>
              <div class="cd-jauge">
                <i
                  :style="{
                    width: `${Math.max(0, Math.round((coussin.profile.hp_current / Math.max(1, coussin.profile.hp_max)) * 100))}%`,
                  }"
                ></i>
              </div>
            </div>

            <ul class="cd-stats">
              <li><span>🧱 Impact</span><b>{{ coussin.profile.atk }}</b></li>
              <li><span>🪶 Moelleux</span><b>{{ coussin.profile.def }}</b></li>
              <li><span>✨ Expérience</span><b>{{ coussin.profile.xp }}</b></li>
              <li v-if="coussin.profile.stat_points > 0" class="cd-dispo">
                <span>🎯 Points à répartir</span><b>{{ coussin.profile.stat_points }}</b>
              </li>
            </ul>

            <p v-if="coussin.profile.stat_points > 0" class="jx-vide">
              Tu as {{ coussin.profile.stat_points }} point(s) à placer.
              Utilise <code>/train</code> sur Discord.
            </p>
          </div>

          <!-- ── Palmarès ── -->
          <h3 class="cd-sous-titre">Ton palmarès</h3>
          <ul class="cd-palmares">
            <li class="gagne"><b>{{ coussin.profile.total_wins }}</b><span>fois assis dessus</span></li>
            <li class="perdu"><b>{{ coussin.profile.total_losses }}</b><span>fois piégé</span></li>
            <li><b>{{ coussin.profile.total_draws }}</b><span>matchs nuls</span></li>
            <li v-if="tauxVictoire !== null"><b>{{ tauxVictoire }} %</b><span>de réussite</span></li>
            <li><b>{{ fmtCoins(coussin.profile.total_stolen) }}</b><span>coins trouvés sous les coussins</span></li>
            <li><b>{{ coussin.profile.chaos_events }}</b><span>fois où le salon a dégénéré</span></li>
            <li v-if="coussin.profile.cowardice_count > 0" class="perdu">
              <b>{{ coussin.profile.cowardice_count }}</b><span>fois resté debout</span>
            </li>
          </ul>

          <!-- ── Inventaire ── -->
          <h3 class="cd-sous-titre">Sous ton coussin</h3>
          <p v-if="!coussin.items.length" class="jx-vide">
            Rien de planqué. Le coffre à coussins s'ouvre avec <code>/shop</code>.
          </p>
          <ul v-else class="cd-objets">
            <li v-for="o in coussin.items" :key="o.item_key" class="cd-objet">
              <span>{{ objet(o.item_key) }}</span>
              <b>×{{ o.quantity }}</b>
            </li>
          </ul>

          <!-- ── Derniers combats ── -->
          <h3 class="cd-sous-titre">Tes dernières bagarres</h3>
          <p v-if="!coussin.combats.length" class="jx-vide">
            Aucune bagarre pour l'instant. Le premier coussin se glisse avec
            <code>/coussin</code> sur Discord.
          </p>
          <ul v-else class="cd-combats">
            <li
              v-for="c in coussin.combats"
              :key="c.id"
              class="cd-combat"
              :class="{ gagne: gagne(c) === true, perdu: gagne(c) === false }"
            >
              <div class="cd-combat-tete">
                <span class="cd-issue">
                  {{ gagne(c) === true ? "Victoire" : gagne(c) === false ? "Défaite" : "Égalité" }}
                </span>
                <span class="cd-adversaire">contre {{ adversaireContexte(c).nom }}</span>
                <span v-if="c.coins_transferred" class="cd-mise">
                  {{ gagne(c) ? "+" : "−" }}{{ fmtCoins(Math.abs(c.coins_transferred)) }} coins
                </span>
                <span v-if="c.resolved_at" class="cd-quand">{{ fmtDate(c.resolved_at) }}</span>
              </div>

              <!-- Les jets de dés : c'est ce qu'on relit pour savoir si on a
                   perdu de peu ou pris une correction. -->
              <div v-if="c.attacker_roll !== null" class="cd-des">
                🎲 {{ c.attacker_name }} {{ c.attacker_roll }}
                — {{ c.defender_name }} {{ c.defender_roll }}
              </div>

              <div v-if="c.special_attack" class="cd-special">
                💫 {{ c.special_attack }}
              </div>
              <div v-if="c.chaos_event" class="cd-chaos">
                🌀 {{ c.chaos_event }}
              </div>
              <p v-if="c.result_message" class="cd-recit">{{ c.result_message }}</p>
            </li>
          </ul>

          <!-- ── Classement ── -->
          <h3 class="cd-sous-titre">Le classement</h3>
          <ol class="jx-rangs">
            <li
              v-for="(r, i) in coussin.ranking"
              :key="r.username + i"
              class="jx-rang"
              :class="{ moi: r.username === coussin.profile.username }"
            >
              <span class="jx-place">{{ i + 1 }}</span>
              <span class="jx-nom">{{ r.username || "Un membre" }}</span>
              <span class="jx-coins">niv. {{ r.level }}</span>
            </li>
          </ol>
        </template>
      </section>

      <!-- Jeu qui ne se joue pas ici et n'a pas de fiche : on l'assume. -->
      <section v-else class="jx-block">
        <h2>{{ jeu.emoji }} {{ jeu.nom }}</h2>
        <p class="jx-vide">{{ jeu.pitch }}</p>
      </section>

      <!-- ── Classement ── -->
      <section class="jx-block">
        <h2>Les plus riches</h2>

        <p v-if="!ranking.length" class="jx-vide">Personne n'a encore de coins.</p>

        <ol v-else class="jx-rangs">
          <li v-for="r in ranking" :key="r.rank" class="jx-rang" :class="{ moi: r.is_me }">
            <span class="jx-place">{{ r.rank }}</span>
            <span class="jx-nom">{{ r.username || "Un membre" }}</span>
            <span class="jx-coins">{{ fmtCoins(r.coins) }}</span>
          </li>
        </ol>
      </section>

      <!-- ── Historique ── -->
      <section class="jx-block">
        <h2>Tes derniers mouvements</h2>

        <p v-if="!history.length" class="jx-vide">
          Aucun mouvement. Ton premier tirage apparaîtra ici.
        </p>

        <ul v-else class="jx-txs">
          <li v-for="t in history" :key="t.id" class="jx-tx">
            <span class="jx-tx-ico" aria-hidden="true">{{ icone(t.source) }}</span>
            <span class="jx-tx-desc">{{ t.description }}</span>
            <span class="jx-tx-montant" :class="t.amount >= 0 ? 'plus' : 'moins'">
              {{ t.amount > 0 ? "+" : "" }}{{ fmtCoins(t.amount) }}
            </span>
            <span class="jx-tx-date">{{ fmtDate(t.created_at) }}</span>
          </li>
        </ul>
      </section>
    </template>
  </div>
</template>

<style scoped>
.jx {
  flex: 1;
  overflow-x: hidden;
  overflow-y: auto;
  padding: clamp(1rem, 3vh, 2rem) clamp(1rem, 4vw, 3rem) 3rem;
  display: flex;
  flex-direction: column;
  gap: clamp(1.75rem, 4vh, 2.5rem);
}

.jx-bar,
.jx-hero,
.jx-block {
  width: 100%;
  max-width: 62rem;
  margin: 0 auto;
}

.jx-bar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 1rem;
}



.jx-solde {
  font-variant-numeric: tabular-nums;
  font-weight: 700;
  font-size: 1.05rem;
  padding: 0.3rem 1rem;
  border-radius: var(--radius-pill);
  background: var(--bg-card);
  border: 1px solid var(--border);
}

/* Le solde change à l'issue du tirage : un bref éclat signale la mise à
   jour, sinon le chiffre bouge sans qu'on le remarque. */
.jx-solde.pulse {
  animation: eclat 0.9s ease-out;
}

@keyframes eclat {
  40% {
    border-color: var(--accent);
    color: #fff;
    transform: scale(1.06);
  }
}





.jx-block h2 {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin: 0 0 1rem;
  font-size: 1.15rem;
}

.jx-count {
  font-size: 0.8rem;
  font-weight: 400;
  color: var(--site-ink-4);
}

.jx-hint {
  text-align: center;
  color: var(--site-ink-4);
}

.jx-vide {
  margin: 0;
  padding: 0.85rem 1.05rem;
  border-radius: var(--radius-lg);
  background: rgba(255, 255, 255, 0.025);
  border: 1px dashed var(--border);
  color: var(--site-ink-4);
  font-size: 0.9rem;
  line-height: 1.5;
}

.jx-vide code {
  font-family: ui-monospace, "Cascadia Mono", Menlo, monospace;
  color: var(--text-secondary);
}

.jx-alerte {
  margin: 0;
  padding: 0.85rem 1.05rem;
  border-radius: var(--radius-lg);
  background: rgba(244, 63, 94, 0.1);
  border: 1px solid rgba(244, 63, 94, 0.35);
  color: #fca5a5;
  font-size: 0.9rem;
}




/* ── La roue ── */
.jx-roue-zone {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 2rem;
  align-items: center;
}

.jx-roue-wrap {
  position: relative;
  width: min(20rem, 60vw);
  aspect-ratio: 1;
}

/* Repère fixe en haut : c'est lui qui désigne la case gagnante. */
.jx-fleche {
  position: absolute;
  top: -0.6rem;
  left: 50%;
  translate: -50% 0;
  z-index: 2;
  width: 0;
  height: 0;
  border-left: 0.7rem solid transparent;
  border-right: 0.7rem solid transparent;
  border-top: 1.1rem solid var(--text-primary);
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.6));
}

.jx-roue {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  border: 4px solid rgba(255, 255, 255, 0.12);
  box-shadow: 0 12px 50px rgba(168, 85, 247, 0.25);
  /* Décélération longue : l'essentiel du plaisir est dans le ralentissement. */
  transition: transform 3s cubic-bezier(0.16, 1, 0.3, 1);
}

.jx-case {
  position: absolute;
  inset: 0;
  display: flex;
  justify-content: center;
  /* Chaque emoji est poussé vers le bord puis redressé, sinon il pencherait
     avec son secteur. */
  padding-top: 0.9rem;
}

.jx-case-in {
  font-size: 1.5rem;
  line-height: 1;
}

.jx-roue-cote {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  align-items: flex-start;
}

.jx-resultat {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  padding: 1rem 1.2rem;
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.jx-resultat.rare {
  border-color: var(--accent);
  box-shadow: 0 0 30px rgba(168, 85, 247, 0.35);
}

.jx-gain {
  font-size: 1.3rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.jx-gain.plus {
  color: var(--success);
}

.jx-gain.moins {
  color: var(--danger);
}

.jx-gain.neutre {
  color: var(--site-ink-4);
  font-size: 1rem;
}

.jx-apres {
  font-size: 0.85rem;
  color: var(--site-ink-3);
}

/* ── Classement ── */
.jx-rangs {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.jx-rang {
  display: grid;
  grid-template-columns: 2.2rem 1fr auto;
  align-items: center;
  gap: 0.8rem;
  padding: 0.6rem 1rem;
  border-radius: var(--radius-md);
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.jx-rang.moi {
  border-color: var(--border-strong);
  background: rgba(168, 85, 247, 0.1);
}

.jx-place {
  font-variant-numeric: tabular-nums;
  font-weight: 700;
  color: var(--site-ink-4);
  text-align: right;
}

.jx-rang.moi .jx-place {
  color: var(--accent);
}

.jx-nom {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.jx-coins {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}

/* ── Historique ── */
.jx-txs {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.jx-tx {
  display: grid;
  grid-template-columns: auto 1fr auto auto;
  align-items: center;
  gap: 0.8rem;
  padding: 0.55rem 1rem;
  border-radius: var(--radius-md);
  background: var(--bg-card);
  border: 1px solid var(--border);
  font-size: 0.9rem;
}

.jx-tx-desc {
  color: var(--site-ink-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.jx-tx-montant {
  font-variant-numeric: tabular-nums;
  font-weight: 700;
}

.jx-tx-montant.plus {
  color: var(--success);
}

.jx-tx-montant.moins {
  color: var(--danger);
}

.jx-tx-date {
  font-size: 0.78rem;
  color: var(--site-ink-4);
  white-space: nowrap;
}

/* ── Carrousel ── */
.jx-carrousel {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.jx-fleche-nav {
  flex: none;
  width: 2rem;
  height: 2rem;
  border-radius: 50%;
  border: 1px solid var(--border);
  background: none;
  color: var(--site-ink-3);
  font-size: 1.1rem;
  line-height: 1;
  cursor: pointer;
}

.jx-fleche-nav:hover {
  border-color: var(--accent);
  color: #fff;
}

.jx-vignettes {
  list-style: none;
  margin: 0;
  padding: 0.2rem;
  display: flex;
  gap: 0.7rem;
  overflow-x: auto;
  /* Chaque vignette s'aligne proprement au défilement plutôt que de rester
     coupée entre deux. */
  scroll-snap-type: x mandatory;
  scrollbar-width: none;
}

.jx-vignettes::-webkit-scrollbar {
  display: none;
}

.jx-vignette {
  scroll-snap-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.3rem;
  min-width: 9rem;
  padding: 0.9rem 1rem;
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font: inherit;
  cursor: pointer;
  transition: border-color 0.15s ease, transform 0.15s ease;
}

.jx-vignette:hover {
  transform: translateY(-2px);
}

/* Le jeu courant se distingue par sa couleur propre : avec plusieurs jeux,
   un simple surlignage ne dirait pas lequel. */
.jx-vignette.active {
  border-color: var(--c);
  box-shadow: 0 0 24px color-mix(in srgb, var(--c) 35%, transparent);
  color: var(--text-primary);
}

.jx-vignette-emoji {
  font-size: 1.8rem;
  line-height: 1;
}

.jx-vignette-nom {
  font-size: 0.85rem;
  font-weight: 600;
  text-align: center;
}

.jx-vignette-tag {
  font-size: 0.68rem;
  padding: 1px 8px;
  border-radius: var(--radius-pill);
  background: rgba(255, 255, 255, 0.08);
  color: var(--site-ink-4);
  white-space: nowrap;
}

/* Jouable des deux côtés : c'est l'information la plus utile de la vignette,
   elle prend la couleur du jeu au lieu de se lire comme une mention grise. */
.jx-vignette-tag.double {
  background: color-mix(in srgb, var(--c) 22%, transparent);
  color: var(--text-primary);
}

.jx-pitch {
  margin: 0.8rem 0 0;
  color: var(--site-ink-3);
  font-size: 0.92rem;
}

/* ── Coussin Piégé ── */
.cd-sous-titre {
  margin: var(--space-2xl) 0 var(--space-lg);
  font-size: 1.02rem;
}

.cd-fiche {
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
  padding: var(--space-xl);
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.cd-classe {
  display: flex;
  align-items: center;
  gap: var(--space-md);
}

.cd-classe-emoji {
  font-size: 2.4rem;
  line-height: 1;
}

.cd-classe-nom {
  font-weight: 700;
  font-size: 1.1rem;
}

.cd-classe-trait {
  color: var(--site-ink-4);
  font-size: 0.86rem;
}

.cd-identite {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-sm);
}

.cd-titre {
  color: var(--accent);
  font-style: italic;
  font-size: 0.9rem;
}

.cd-niveau {
  margin-left: auto;
  padding: 2px 12px;
  border-radius: var(--radius-pill);
  background: var(--accent-bg);
  font-size: 0.85rem;
  font-weight: 600;
}

.cd-pv-ligne {
  display: flex;
  justify-content: space-between;
  font-size: 0.85rem;
  color: var(--site-ink-3);
  margin-bottom: var(--space-xs);
}

.cd-jauge {
  height: 10px;
  border-radius: var(--radius-pill);
  background: rgba(255, 255, 255, 0.07);
  overflow: hidden;
}

/* Du vert au rouge selon ce qui reste : la couleur dit l'état avant que le
   chiffre soit lu. */
.cd-jauge i {
  display: block;
  height: 100%;
  border-radius: var(--radius-pill);
  background: linear-gradient(90deg, var(--danger), var(--accent-warm), var(--success));
  background-size: 300% 100%;
  background-position: right;
  transition: width var(--transition-base);
}

.cd-stats {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
  gap: var(--space-sm);
}

.cd-stats li {
  display: flex;
  justify-content: space-between;
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-md);
  background: rgba(255, 255, 255, 0.03);
  font-size: 0.88rem;
}

.cd-stats b {
  font-variant-numeric: tabular-nums;
}

.cd-stats .cd-dispo {
  border: 1px solid var(--border-strong);
}

.cd-palmares {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
  gap: var(--space-md);
}

.cd-palmares li {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: var(--space-md);
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.cd-palmares b {
  font-size: 1.4rem;
  font-variant-numeric: tabular-nums;
}

.cd-palmares span {
  color: var(--site-ink-4);
  font-size: 0.78rem;
  text-align: center;
}

.cd-palmares .gagne b {
  color: var(--success);
}

.cd-palmares .perdu b {
  color: var(--danger);
}

.cd-objets {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-sm);
}

.cd-objet {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-lg);
  border-radius: var(--radius-pill);
  background: var(--bg-card);
  border: 1px solid var(--border);
  font-size: 0.88rem;
}

.cd-combats {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.cd-combat {
  padding: var(--space-md) var(--space-lg);
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border);
  /* Le bord gauche porte l'issue : lisible en diagonale, sans lire le texte. */
  border-left: 3px solid var(--site-off);
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}

.cd-combat.gagne {
  border-left-color: var(--success);
}

.cd-combat.perdu {
  border-left-color: var(--danger);
}

.cd-combat-tete {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-sm);
}

.cd-issue {
  font-weight: 700;
}

.cd-combat.gagne .cd-issue {
  color: var(--success);
}

.cd-combat.perdu .cd-issue {
  color: var(--danger);
}

.cd-adversaire {
  color: var(--site-ink-3);
  font-size: 0.9rem;
}

.cd-mise {
  margin-left: auto;
  font-variant-numeric: tabular-nums;
  font-weight: 600;
  font-size: 0.88rem;
}

.cd-quand {
  font-size: 0.76rem;
  color: var(--site-ink-4);
  white-space: nowrap;
}

.cd-des,
.cd-special,
.cd-chaos {
  font-size: 0.84rem;
  color: var(--site-ink-3);
  font-variant-numeric: tabular-nums;
}

.cd-chaos {
  color: var(--accent);
}

.cd-recit {
  margin: var(--space-xs) 0 0;
  font-size: 0.86rem;
  line-height: 1.5;
  color: var(--site-ink-4);
  font-style: italic;
}

@media (max-width: 760px) {
  .jx-roue-zone {
    grid-template-columns: 1fr;
    justify-items: center;
  }

  .jx-roue-cote {
    align-items: center;
    text-align: center;
  }

  .jx-tx {
    grid-template-columns: auto 1fr auto;
  }

  .jx-tx-date {
    display: none;
  }
}

@media (prefers-reduced-motion: reduce) {
  .jx-roue {
    transition: none;
  }

  .jx-solde.pulse {
    animation: none;
  }
}
</style>
