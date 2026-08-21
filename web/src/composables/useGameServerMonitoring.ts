import { computed, onUnmounted, ref, watch, type Ref } from "vue";
import { registerChartJs } from "@/utils/chartjs";
import {
  nexusGamesService,
  type GameServerStats,
} from "@/services/nexusGamesService";

/**
 * Surveillance d'un serveur de jeu : chiffres en direct + courbes d'historique.
 *
 * Extraites de la page de détail, où elles occupaient la moitié du fichier.
 *
 *   - les statistiques ne sont rafraîchies que si le serveur tourne :
 *     interroger Docker toutes les 5 s pour un conteneur arrêté ne renverrait
 *     que des zéros, en payant une requête à chaque fois ;
 *   - les CHIFFRES restent en direct (une valeur instantanée doit être vive),
 *     les COURBES viennent de l'historique enregistré côté serveur — elles ne
 *     dépendent donc plus du temps passé sur la page, et survivent à un
 *     rechargement ;
 *   - une tranche sans mesure n'est pas tracée à zéro : elle reste vide, et
 *     le trou dit « le serveur était éteint » — ce qu'une ligne au sol
 *     cacherait.
 */
export function useGameServerMonitoring(
  guildId: () => string | undefined,
  serverId: () => string | undefined,
  isRunning: Ref<boolean>,
) {
  registerChartJs();

  // ── Statistiques instantanées, rafraîchies toutes les 5 s en direct ──
  const stats = ref<GameServerStats | null>(null);

  async function refreshStats() {
    const g = guildId();
    const s = serverId();
    if (!g || !s || !isRunning.value) {
      stats.value = null;
      return;
    }
    stats.value = await nexusGamesService
      .stats(g, s)
      .catch(() => null);
  }

  let statsTimer: ReturnType<typeof setInterval> | null = null;
  function syncStatsTimer() {
    if (statsTimer) {
      clearInterval(statsTimer);
      statsTimer = null;
    }
    if (isRunning.value) {
      void refreshStats();
      statsTimer = setInterval(refreshStats, 5000);
    } else {
      stats.value = null;
    }
  }
  watch(isRunning, syncStatsTimer, { immediate: true });
  onUnmounted(() => statsTimer && clearInterval(statsTimer));

  // ── Historique : courbes sur la plage choisie, chargées côté serveur ──
  const cpuHistory = ref<number[]>([]);
  const ramHistory = ref<number[]>([]);
  /// Débits relevés, en Ko/s. Le débit, pas le compteur cumulé : une courbe
  /// de total ne fait que monter et ne montre aucune saturation.
  const netRxHistory = ref<number[]>([]);
  const netTxHistory = ref<number[]>([]);
  /// Temps de réponse du jeu, en ms — la mesure qui suit le lag ressenti.
  const latencyHistory = ref<number[]>([]);
  /// Joueurs connectés au fil du temps. La carte n'affichait qu'un chiffre —
  /// « 0 » — qui ne disait rien de la soirée écoulée.
  const playersHistory = ref<(number | null)[]>([]);
  /// Heure de chaque point, partagée par tous les graphiques : sans axe des
  /// temps, on ne sait pas si un pic date d'une minute ou d'une demi-heure.
  const timeLabels = ref<string[]>([]);
  const historiqueEnCours = ref(false);

  /// Plages proposées. Les serveurs redémarrent chaque nuit : c'est la journée
  /// qui est l'unité d'observation utile, pas la demi-heure.
  const PLAGES = [
    { libelle: "30 min", secondes: 1800 },
    { libelle: "2 h", secondes: 7200 },
    { libelle: "6 h", secondes: 21600 },
    { libelle: "24 h", secondes: 86400 },
    { libelle: "7 j", secondes: 604800 },
  ] as const;
  const plageChoisie = ref<number>(1800);
  /// Pas réellement appliqué par l'API, affiché à l'écran : elle élargit le
  /// pas quand la demande produirait trop de points, et une courbe dégrossie
  /// sans explication passerait pour une perte de mesures.
  const pasApplique = ref<number>(0);

  /// Charge les courbes pour la plage choisie.
  async function loadHistorique() {
    const g = guildId();
    const s = serverId();
    if (!g || !s) return;
    historiqueEnCours.value = true;
    const historique = await nexusGamesService
      .perfHistory(g, s, plageChoisie.value)
      .catch(() => null);
    historiqueEnCours.value = false;
    if (!historique) return;

    pasApplique.value = historique.step_secs;
    const points = historique.points;

    // Sur une journée entière, l'heure seule suffit ; sur trente minutes, il
    // faut les minutes. Afficher le jour partout rendrait l'axe illisible.
    const longuePlage = plageChoisie.value > 86400;
    timeLabels.value = points.map((p) => {
      const d = new Date(p.horodatage);
      return longuePlage
        ? d.toLocaleDateString("fr-FR", { day: "2-digit", month: "2-digit" }) +
            " " +
            d.toLocaleTimeString("fr-FR", { hour: "2-digit" })
        : d.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
    });

    const arrondi = (v: number | null, facteur = 10) =>
      v === null ? null : Math.round(v * facteur) / facteur;

    cpuHistory.value = points.map((p) => arrondi(p.cpu_percent)) as number[];
    ramHistory.value = points.map((p) =>
      p.memory_used_mb === null || !p.memory_limit_mb
        ? null
        : Math.round((p.memory_used_mb / p.memory_limit_mb) * 1000) / 10,
    ) as number[];
    netRxHistory.value = points.map((p) =>
      p.net_rx_bytes_per_sec === null ? null : arrondi(p.net_rx_bytes_per_sec / 1024),
    ) as number[];
    netTxHistory.value = points.map((p) =>
      p.net_tx_bytes_per_sec === null ? null : arrondi(p.net_tx_bytes_per_sec / 1024),
    ) as number[];
    latencyHistory.value = points.map((p) => p.rcon_latency_ms) as number[];
    playersHistory.value = points.map((p) => p.player_count);
  }

  function changerPlage(secondes: number) {
    plageChoisie.value = secondes;
    void loadHistorique();
  }

  /// Libellé du pas appliqué, pour que l'écran dise à quoi correspond un point.
  const pasLisible = computed(() => {
    const s = pasApplique.value;
    if (!s) return "";
    if (s >= 3600) return `${Math.round(s / 3600)} h`;
    if (s >= 60) return `${Math.round(s / 60)} min`;
    return `${s} s`;
  });

  // ── Données et options des graphiques ──

  /// Axe des temps commun. Les étiquettes sont espacées automatiquement par
  /// Chart.js (`autoSkip`) : les afficher toutes rendrait l'axe illisible sur
  /// une carte étroite.
  const axeTemps = {
    display: true,
    grid: { display: false },
    ticks: {
      color: "rgba(255, 255, 255, 0.45)",
      maxRotation: 0,
      autoSkip: true,
      maxTicksLimit: 4,
      font: { size: 9 },
    },
  };

  /// Options des graphes en POURCENTAGE : l'échelle 0-100 est fixe, sinon une
  /// variation de 2 % remplirait la carte et ferait croire à une saturation.
  const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    animation: { duration: 0 },
    scales: {
      y: {
        min: 0,
        max: 100,
        grid: { color: "rgba(255, 255, 255, 0.1)" },
        ticks: { color: "rgba(255, 255, 255, 0.5)" },
      },
      x: axeTemps,
    },
    plugins: { legend: { display: false } },
    elements: { point: { radius: 0 } },
  };

  /// Options des graphes SANS plafond connu — débit, temps de réponse. L'échelle
  /// s'adapte aux valeurs : imposer un maximum arbitraire écraserait la courbe
  /// ou masquerait un pic.
  const chartOptionsAuto = {
    responsive: true,
    maintainAspectRatio: false,
    animation: { duration: 0 },
    scales: {
      y: {
        min: 0,
        grid: { color: "rgba(255, 255, 255, 0.1)" },
        ticks: { color: "rgba(255, 255, 255, 0.5)", maxTicksLimit: 5 },
      },
      x: axeTemps,
    },
    plugins: { legend: { display: false } },
    elements: { point: { radius: 0 } },
  };

  /// Mêmes options, mais avec la légende : le graphe réseau porte deux courbes,
  /// et sans légende on ne sait pas laquelle est le trafic reçu.
  const chartOptionsReseau = {
    ...chartOptionsAuto,
    plugins: {
      legend: {
        display: true,
        labels: { color: "rgba(255, 255, 255, 0.6)", boxWidth: 10, font: { size: 10 } },
      },
    },
  };

  const cpuChartData = computed(() => ({
    labels: [...timeLabels.value],
    datasets: [
      {
        label: "CPU (%)",
        backgroundColor: "rgba(52, 152, 219, 0.2)",
        borderColor: "#3498db",
        data: [...cpuHistory.value],
        fill: true,
        tension: 0.4,
        borderWidth: 2,
      },
    ],
  }));

  const ramChartData = computed(() => ({
    labels: [...timeLabels.value],
    datasets: [
      {
        label: "RAM (%)",
        backgroundColor: "rgba(241, 196, 15, 0.2)",
        borderColor: "#f1c40f",
        data: [...ramHistory.value],
        fill: true,
        tension: 0.4,
        borderWidth: 2,
      },
    ],
  }));

  /// Deux courbes sur le même graphe : reçu et envoyé se lisent l'un par
  /// rapport à l'autre. Un serveur de jeu émet bien plus qu'il ne reçoit —
  /// c'est l'écart entre les deux qui signale une saturation en émission.
  const netChartData = computed(() => ({
    labels: [...timeLabels.value],
    datasets: [
      {
        label: "Reçu (Ko/s)",
        backgroundColor: "rgba(46, 204, 113, 0.15)",
        borderColor: "#2ecc71",
        data: [...netRxHistory.value],
        fill: true,
        tension: 0.4,
        borderWidth: 2,
      },
      {
        label: "Envoyé (Ko/s)",
        backgroundColor: "rgba(155, 89, 182, 0.15)",
        borderColor: "#9b59b6",
        data: [...netTxHistory.value],
        fill: true,
        tension: 0.4,
        borderWidth: 2,
      },
    ],
  }));

  const latencyChartData = computed(() => ({
    labels: [...timeLabels.value],
    datasets: [
      {
        label: "Temps de réponse (ms)",
        backgroundColor: "rgba(241, 196, 15, 0.15)",
        borderColor: "#f1c40f",
        data: [...latencyHistory.value],
        fill: true,
        tension: 0.4,
        borderWidth: 2,
      },
    ],
  }));

  /// Joueurs connectés au fil du temps. C'est la courbe la plus parlante du
  /// lot : elle dit quand le serveur est utilisé, et à quelle heure il ne
  /// l'est plus.
  ///
  /// `spanGaps: false` laisse les trous visibles : une tranche sans mesure est
  /// un serveur éteint, pas un serveur désert.
  const playersChartData = computed(() => ({
    labels: [...timeLabels.value],
    datasets: [
      {
        label: "Joueurs",
        backgroundColor: "rgba(52, 152, 219, 0.2)",
        borderColor: "#3498db",
        data: [...playersHistory.value],
        fill: true,
        tension: 0.3,
        borderWidth: 2,
        spanGaps: false,
      },
    ],
  }));

  /// Échelle entière : un demi-joueur n'existe pas, et Chart.js graduerait
  /// volontiers en 0,5 sur un serveur qui n'en a jamais plus de deux.
  const chartOptionsJoueurs = computed(() => ({
    ...chartOptionsAuto,
    scales: {
      ...chartOptionsAuto.scales,
      y: {
        ...chartOptionsAuto.scales.y,
        beginAtZero: true,
        ticks: { ...chartOptionsAuto.scales.y.ticks, precision: 0, stepSize: 1 },
      },
    },
  }));

  return {
    stats,
    refreshStats,
    // Historique
    historiqueEnCours,
    PLAGES,
    plageChoisie,
    pasApplique,
    pasLisible,
    timeLabels,
    changerPlage,
    loadHistorique,
    // Graphiques
    chartOptions,
    chartOptionsAuto,
    chartOptionsReseau,
    chartOptionsJoueurs,
    cpuChartData,
    ramChartData,
    netChartData,
    latencyChartData,
    playersChartData,
  };
}

/**
 * Rend un volume d'octets lisible : 2 300 000 000 ne se lit pas, « 2,14 Go »
 * oui.
 */
export function volume(octets: number | null | undefined): string {
  const v = Number(octets) || 0;
  if (v < 1024) return `${v} o`;
  if (v < 1024 * 1024) return `${(v / 1024).toFixed(1)} Ko`;
  if (v < 1024 * 1024 * 1024) return `${(v / (1024 * 1024)).toFixed(1)} Mo`;
  return `${(v / (1024 * 1024 * 1024)).toFixed(2)} Go`;
}

/** Rend un débit lisible : 2 300 000 o/s ne se lit pas, « 2,19 Mo/s » oui. */
export function debit(octetsParSeconde: number): string {
  const v = Number(octetsParSeconde) || 0;
  if (v < 1024) return `${v} o/s`;
  if (v < 1024 * 1024) return `${(v / 1024).toFixed(1)} Ko/s`;
  return `${(v / (1024 * 1024)).toFixed(2)} Mo/s`;
}
