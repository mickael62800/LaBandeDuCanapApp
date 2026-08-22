import { describe, expect, it, vi, beforeEach } from "vitest";

import { useGameServerSchedule } from "./useGameServerSchedule";
import type { ServerSchedule } from "@/services/nexusGamesService";

/**
 * Les deux systèmes de pilotage s'excluent, et l'API tranche. Ce que cet écran
 * doit garantir, lui, c'est qu'il ne propose jamais une configuration que
 * l'API refusera — activer la permanence sans cadence, notamment.
 */

const getScheduleRanges = vi.fn();
const saveScheduleRanges = vi.fn();

vi.mock("@/services/nexusGamesService", () => ({
  nexusGamesService: {
    getScheduleRanges: (...args: unknown[]) => getScheduleRanges(...args),
    saveScheduleRanges: (...args: unknown[]) => saveScheduleRanges(...args),
  },
}));

vi.mock("./useToast", () => ({
  useToast: () => ({ success: vi.fn(), error: vi.fn() }),
}));

/** Réponse d'API minimale : seuls les champs lus par le composable comptent. */
function reponse(partiel: Partial<ServerSchedule> = {}): ServerSchedule {
  return {
    enabled: false,
    mode: "ranges",
    timezone: "Europe/Paris",
    ranges: [],
    warn_minutes: 10,
    next_opening: null,
    disabled_restart_keys: [],
    restart_interval_hours: null,
    restart_anchor_minute: 0,
    next_restart: null,
    restart_interval_choices: [1, 2, 3, 4, 6, 8, 12, 24],
    ...partiel,
  };
}

function monter() {
  return useGameServerSchedule(
    () => "guilde-1",
    () => "serveur-1",
  );
}

describe("useGameServerSchedule", () => {
  beforeEach(() => {
    getScheduleRanges.mockReset();
    saveScheduleRanges.mockReset();
  });

  it("part sur les plages horaires tant que rien n'est configuré", () => {
    const s = monter();
    expect(s.mode.value).toBe("ranges");
    expect(s.estPermanence.value).toBe(false);
  });

  it("relit le mode enregistré", async () => {
    getScheduleRanges.mockResolvedValue(
      reponse({ mode: "restart", enabled: true, restart_interval_hours: 6 }),
    );
    const s = monter();
    await s.load();
    expect(s.estPermanence.value).toBe(true);
    expect(s.restartIntervalHours.value).toBe(6);
  });

  it("propose une cadence en passant en permanence", () => {
    // Sans cadence, l'API refuserait l'activation : l'administrateur verrait
    // une erreur là où il attend un réglage.
    const s = monter();
    s.choisirMode("restart");
    expect(s.restartIntervalHours.value).not.toBeNull();
    // Un quart d'heure : le préavis qui a du sens pour un redémarrage.
    expect(s.warn.value).toBe(15);
  });

  it("ne réécrase pas une cadence déjà choisie", async () => {
    getScheduleRanges.mockResolvedValue(
      reponse({ mode: "restart", restart_interval_hours: 12, warn_minutes: 30 }),
    );
    const s = monter();
    await s.load();
    s.choisirMode("restart");
    expect(s.restartIntervalHours.value).toBe(12);
    expect(s.warn.value).toBe(30);
  });

  it("revient aux plages sans perdre les créneaux saisis", () => {
    const s = monter();
    s.ajouterPlage();
    s.choisirMode("restart");
    s.choisirMode("ranges");
    expect(s.estPermanence.value).toBe(false);
    expect(s.ranges.value).toHaveLength(1);
  });

  it("envoie le mode et la cadence à l'enregistrement", async () => {
    saveScheduleRanges.mockResolvedValue(
      reponse({ mode: "restart", enabled: true, restart_interval_hours: 3 }),
    );
    const s = monter();
    s.choisirMode("restart");
    s.restartIntervalHours.value = 3;
    s.enabled.value = true;
    await s.save();

    const envoye = saveScheduleRanges.mock.calls[0][2];
    expect(envoye.mode).toBe("restart");
    expect(envoye.restart_interval_hours).toBe(3);
  });

  it("convertit les plages en minutes depuis minuit", async () => {
    saveScheduleRanges.mockResolvedValue(reponse());
    const s = monter();
    s.ranges.value = [{ start: "19:30", end: "23:00" }];
    await s.save();

    const envoye = saveScheduleRanges.mock.calls[0][2];
    expect(envoye.ranges).toEqual([{ start_minute: 1170, end_minute: 1380 }]);
  });

  it("retient ce que le serveur a réellement enregistré", async () => {
    // Il borne le préavis et recalcule les échéances : supposer que l'envoi a
    // été retenu tel quel afficherait un réglage qui n'existe pas.
    saveScheduleRanges.mockResolvedValue(
      reponse({ mode: "restart", warn_minutes: 120, restart_interval_hours: 24 }),
    );
    const s = monter();
    s.warn.value = 9999;
    await s.save();
    expect(s.warn.value).toBe(120);
    expect(s.restartIntervalHours.value).toBe(24);
  });

  it("garde le formulaire tel quel si la lecture échoue", async () => {
    getScheduleRanges.mockRejectedValue(new Error("API muette"));
    const s = monter();
    s.timezone.value = "Europe/Lisbon";
    await s.load();
    expect(s.timezone.value).toBe("Europe/Lisbon");
  });

  it("n'affiche une échéance que lorsqu'il y en a une", async () => {
    const s = monter();
    expect(s.prochainRedemarrage.value).toBeNull();
    expect(s.prochaineOuverture.value).toBeNull();

    getScheduleRanges.mockResolvedValue(
      reponse({ mode: "restart", next_restart: "2026-08-19T13:00:00Z" }),
    );
    await s.load();
    expect(s.prochainRedemarrage.value).not.toBeNull();
  });

  it("laisse le serveur dicter les cadences proposées", async () => {
    // La liste ne doit pas pouvoir diverger de ce que l'API accepte.
    getScheduleRanges.mockResolvedValue(
      reponse({ restart_interval_choices: [2, 4] }),
    );
    const s = monter();
    await s.load();
    expect(s.restartIntervalChoices.value).toEqual([2, 4]);
  });

  it("ne fait rien sans guilde ni serveur", async () => {
    const s = useGameServerSchedule(
      () => null,
      () => null,
    );
    await s.load();
    await s.save();
    expect(getScheduleRanges).not.toHaveBeenCalled();
    expect(saveScheduleRanges).not.toHaveBeenCalled();
  });
});
