import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { effectScope, ref, type EffectScope } from "vue";

const statsMock = vi.hoisted(() => vi.fn());
const perfHistoryMock = vi.hoisted(() => vi.fn());
vi.mock("@/services/nexusGamesService", () => ({
  nexusGamesService: {
    stats: (...args: unknown[]) => statsMock(...(args as [])),
    perfHistory: (...args: unknown[]) => perfHistoryMock(...(args as [])),
  },
}));

const registerChartJs = vi.hoisted(() => vi.fn());
vi.mock("@/utils/chartjs", () => ({ registerChartJs }));

import { debit, useGameServerMonitoring, volume } from "./useGameServerMonitoring";
import type { HistoriqueSurveillance } from "@/services/nexusGamesService";

/** Point d'historique minimal : tout est null sauf ce que le test fixe. */
function point(partiel: Partial<HistoriqueSurveillance["points"][number]> = {}) {
  return {
    horodatage: "2026-08-21T14:35:00Z",
    cpu_percent: null,
    memory_used_mb: null,
    memory_limit_mb: null,
    rcon_latency_ms: null,
    net_rx_bytes_per_sec: null,
    net_tx_bytes_per_sec: null,
    player_count: null,
    ...partiel,
  };
}

function historique(points: HistoriqueSurveillance["points"], stepSecs = 60) {
  return { points, range_secs: 1800, step_secs: stepSecs } as unknown as HistoriqueSurveillance;
}

/** Le watch du composable se resout en microtask : on attend un battement. */
async function tick() {
  await vi.advanceTimersByTimeAsync(0);
}

describe("useGameServerMonitoring", () => {
  let scope: EffectScope | null = null;

  beforeEach(() => {
    vi.useFakeTimers();
    statsMock.mockReset().mockResolvedValue({ cpu_percent: 12, memory_used_mb: 500 });
    perfHistoryMock.mockReset().mockResolvedValue(historique([point()]));
    registerChartJs.mockClear();
  });

  afterEach(() => {
    scope?.stop(); // déclenche les onUnmounted du composable (clearInterval)
    scope = null;
    vi.useRealTimers();
  });

  function monter(
    guild: () => string | null,
    server: () => string | null,
    isRunning = ref(false),
  ) {
    const s = effectScope(true);
    scope = s;
    return s.run(() => useGameServerMonitoring(guild, server, isRunning))!;
  }

  it("sans guilde ni serveur : aucune requete, stats a null", async () => {
    const m = monter(() => null, () => "srv-1");
    await tick();

    expect(m.stats.value).toBeNull();
    expect(statsMock).not.toHaveBeenCalled();
  });

  it("serveur arrete : pas de polling, stats a null", async () => {
    const m = monter(() => "g1", () => "srv-1"); // isRunning = false par defaut
    await vi.advanceTimersByTimeAsync(60_000);

    expect(m.stats.value).toBeNull();
    expect(statsMock).not.toHaveBeenCalled();
  });

  it("serveur en marche : lecture immediate puis toutes les 5 s", async () => {
    const m = monter(() => "g1", () => "srv-1", ref(true));
    await tick(); // laisse le refresh immediat se resoudre

    expect(statsMock).toHaveBeenCalledTimes(1);
    expect(m.stats.value).toEqual({ cpu_percent: 12, memory_used_mb: 500 });

    await vi.advanceTimersByTimeAsync(5000);
    await vi.advanceTimersByTimeAsync(5000);
    expect(statsMock).toHaveBeenCalledTimes(3);
  });

  it("une erreur de stats ne casse pas le cycle (null, puis re-essai)", async () => {
    const m = monter(() => "g1", () => "srv-1", ref(true));
    await tick();
    expect(m.stats.value).toEqual({ cpu_percent: 12, memory_used_mb: 500 });

    statsMock.mockRejectedValueOnce(new Error("docker down"));
    await vi.advanceTimersByTimeAsync(5000);
    expect(m.stats.value).toBeNull(); // .catch(() => null)

    await vi.advanceTimersByTimeAsync(5000);
    expect(statsMock).toHaveBeenCalledTimes(3); // le timer continue de tourner
  });

  it("arreter le serveur stoppe le polling et efface les stats", async () => {
    const isRunning = ref(true);
    const m = monter(() => "g1", () => "srv-1", isRunning);
    await tick(); // lecture initiale enregistree

    isRunning.value = false;
    await tick(); // le watch se resout : clearInterval + stats=null
    expect(m.stats.value).toBeNull();

    const appels = statsMock.mock.calls.length;
    await vi.advanceTimersByTimeAsync(20_000);
    expect(statsMock.mock.calls.length).toBe(appels); // plus aucun appel periodique
  });

  it("arreter puis re-demarrer : un seul cycle de polling a la fois", async () => {
    const isRunning = ref(true);
    monter(() => "g1", () => "srv-1", isRunning);
    await tick(); // lecture initiale enregistree

    statsMock.mockClear();
    isRunning.value = false;
    await tick(); // arret : plus de timer, aucune requete
    expect(statsMock).not.toHaveBeenCalled();

    isRunning.value = true;
    await tick(); // redemarrage : refresh immediat + un seul setInterval
    expect(statsMock).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(5000);
    expect(statsMock).toHaveBeenCalledTimes(2); // 1 immediate + 1 tick, pas de doublon
  });

  it("loadHistorique sans guilde/serveur : aucun appel, historiqueEnCours reste faux", async () => {
    const m = monter(() => null, () => "srv-1");
    await m.loadHistorique();

    expect(perfHistoryMock).not.toHaveBeenCalled();
    expect(m.historiqueEnCours.value).toBe(false);
  });

  it("charge l'historique : courbes arrondies, trous conserves en null", async () => {
    perfHistoryMock.mockResolvedValue(
      historique([
        point({ cpu_percent: 12.3456789 }), // arrondi a 0.1 -> 12.3
        point({ memory_used_mb: 512, memory_limit_mb: 1024 }), // 50 %
        point({ net_rx_bytes_per_sec: 2048, net_tx_bytes_per_sec: null }), // 2 Ko/s / trou
        point({ rcon_latency_ms: 37.654 }), // conservee telle quelle (pas d'arrondi)
        point({ player_count: 3 }),
      ]),
    );

    const m = monter(() => "g1", () => "srv-1");
    await m.loadHistorique();

    expect(perfHistoryMock).toHaveBeenCalledWith("g1", "srv-1", 1800); // plage par defaut : 30 min
    expect(m.historiqueEnCours.value).toBe(false);
    expect(m.pasApplique.value).toBe(60);

    const data = (c: { datasets: Array<{ data: unknown[] }> }) => c.datasets[0].data;

    // CPU arrondi a 0.1 ; les tranches sans mesure restent des TROUS, pas du zero.
    expect(data(m.cpuChartData.value)).toEqual([12.3, null, null, null, null]);
    // RAM en % de la limite ; sans limite connue -> trou (null), pas 0 %.
    expect(data(m.ramChartData.value)).toEqual([null, 50, null, null, null]);

    const reseau = m.netChartData.value;
    expect(reseau.datasets.map((d) => d.label)).toEqual(["Reçu (Ko/s)", "Envoyé (Ko/s)"]);
    expect(data(m.netChartData.value)).toEqual([null, null, 2, null, null]); // 2048 o -> 2 Ko/s
    expect(reseau.datasets[1].data).toEqual([null, null, null, null, null]);

    expect(data(m.latencyChartData.value)).toEqual([null, null, null, 37.654, null]);
    const joueurs = m.playersChartData.value;
    expect(joueurs.datasets.map((d) => d.label)).toEqual(["Joueurs"]);
    expect(data(joueurs)).toEqual([null, null, null, null, 3]);

    // Plage <= 24 h : heures:minutes seulement (fuseau local du testeur).
    const d = new Date("2026-08-21T14:35:00Z");
    expect(m.timeLabels.value[0]).toBe(
      d.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" }),
    );

    // Les donnees des graphiques sont des COPIES : muter le computed ne doit pas
    // toucher l'etat interne du composable.
    const cpu = m.cpuChartData.value;
    expect(cpu.datasets[0].label).toBe("CPU (%)");
    expect(cpu.labels.length).toBe(5);
  });

  it("plage > 24 h : l'axe affiche le jour, pas les minutes", async () => {
    const m = monter(() => "g1", () => "srv-1");
    await tick(); // laisse eventuellement se resoudre une initiale en attente

    m.changerPlage(604800); // 7 j > 86400 s -> jour + heure
    await vi.advanceTimersByTimeAsync(0);

    const d = new Date("2026-08-21T14:35:00Z");
    expect(m.timeLabels.value[0]).toBe(
      d.toLocaleDateString("fr-FR", { day: "2-digit", month: "2-digit" }) +
        " " +
        d.toLocaleTimeString("fr-FR", { hour: "2-digit" }),
    );
  });

  it("echec de l'historique : historiqueEnCours remis a faux, courbes vides", async () => {
    perfHistoryMock.mockRejectedValue(new Error("503"));

    const m = monter(() => "g1", () => "srv-1");
    await m.loadHistorique();

    expect(m.historiqueEnCours.value).toBe(false);
    expect(m.timeLabels.value).toEqual([]);
    expect(m.cpuChartData.value.datasets[0].data).toEqual([]);
  });

  it("changerPlage met a jour la plage et relance le chargement", async () => {
    const m = monter(() => "g1", () => "srv-1");
    await tick(); // laisse eventuellement se resoudre l'initiale

    perfHistoryMock.mockClear();
    m.changerPlage(7200);
    expect(m.plageChoisie.value).toBe(7200);
    await vi.advanceTimersByTimeAsync(0);
    expect(perfHistoryMock).toHaveBeenCalledWith("g1", "srv-1", 7200);

    // PLAGES expose les cinq tranches, de la plus courte a la plus longue.
    expect(m.PLAGES.map((p) => p.secondes)).toEqual([1800, 7200, 21600, 86400, 604800]);
  });

  it("pasLisible : heures / minutes / secondes", () => {
    const m = monter(() => "g1", () => "srv-1");
    expect(m.pasLisible.value).toBe(""); // pas encore d'historique charge

    m.pasApplique.value = 7200;
    expect(m.pasLisible.value).toBe("2 h");
    m.pasApplique.value = 300;
    expect(m.pasLisible.value).toBe("5 min");
    m.pasApplique.value = 45;
    expect(m.pasLisible.value).toBe("45 s");
  });

  it("expose des options de graphiques coherentes (echelles, legende reseau)", () => {
    const m = monter(() => "g1", () => "srv-1");

    // Pourcentage : plafond fixe a 100.
    expect(m.chartOptions.scales.y.max).toBe(100);
    expect(m.chartOptions.plugins.legend.display).toBe(false);

    // Sans plafond connu : echelle libre, legende masquee...
    expect(m.chartOptionsAuto.scales.y.min).toBe(0);
    expect((m.chartOptionsAuto as { scales: { y: { max?: number } } }).scales.y.max).toBeUndefined();

    // ...sauf le graphe reseau qui porte deux courbes.
    expect(m.chartOptionsReseau.plugins.legend.display).toBe(true);

    // Joueurs : echelle entiere (pas de demi-joueur), legende masquee.
    const y = m.chartOptionsJoueurs.value.scales.y as { beginAtZero: boolean; ticks: Record<string, unknown> };
    expect(y.beginAtZero).toBe(true);
    expect(y.ticks.precision).toBe(0);
    expect(y.ticks.stepSize).toBe(1);

    // Le graphe latence porte une courbe unique.
    expect(m.latencyChartData.value.datasets.map((d) => d.label)).toEqual([
      "Temps de r\u00e9ponse (ms)",
    ]);
  });
});

describe("volume / debit", () => {
  it("rend les volumes lisibles sur toute l'echelle", () => {
    expect(volume(512)).toBe("512 o");
    expect(volume(null)).toBe("0 o"); // null -> 0, pas d'exception
    expect(volume(undefined)).toBe("0 o");
    expect(volume(2048)).toBe("2.0 Ko");
    expect(volume(3 * 1024 * 1024 + 512 * 1024)).toBe("3.5 Mo");
    expect(volume(2_300_000_000)).toBe(`${(2_300_000_000 / (1024 ** 3)).toFixed(2)} Go`);
  });

  it("rend les debits lisibles", () => {
    expect(debit(900)).toBe("900 o/s");
    expect(debit(0)).toBe("0 o/s"); // NaN/0 -> 0, pas d'exception
    expect(debit(2048)).toBe("2.0 Ko/s");
    expect(debit(2_300_000)).toBe(`${(2_300_000 / (1024 ** 2)).toFixed(2)} Mo/s`);
  });
});
