import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  nexusGet: vi.fn(),
  nexusPost: vi.fn(),
  nexusPut: vi.fn(),
  nexusDelete: vi.fn(),
}));

vi.mock("@/api/nexusHttp", () => mocks);

import { adresseServeur, nexusGamesService } from "./nexusGamesService";
import type { GameServer } from "./nexusGamesService";

describe("adresseServeur", () => {
  const base: GameServer = {
    id: "s1",
    guild_id: "g1",
    template_id: "t1",
    name: "S",
    status: "running",
    host_port: null,
    rcon_port: null,
    allocated_memory_mb: 2048,
    cpu_limit: null,
    owner_user_id: "u1",
    last_active_at: null,
    last_player_count: 0,
    last_error: null,
    created_at: "",
    started_at: null,
    stopped_at: null,
    text_channel_id: null,
    voice_channel_id: null,
    ip_reveal_at: null,
    ip_revealed: false,
    public_host: "play.example.com",
  };

  it("compose l'adresse quand hote et port sont presents", () => {
    expect(adresseServeur({ ...base, host_port: 25565 })).toBe(
      "play.example.com:25565",
    );
  });

  it("renvoie null sans port (conteneur jamais demarre)", () => {
    expect(adresseServeur(base)).toBeNull();
  });

  it("renvoie null sans hote public", () => {
    expect(
      adresseServeur({ ...base, host_port: 25565, public_host: null }),
    ).toBeNull();
  });
});

describe("nexusGamesService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("listServers interroge la liste des serveurs de la guilde", async () => {
    const reponse = [{ id: "s1" }];
    mocks.nexusGet.mockResolvedValue(reponse);

    await expect(nexusGamesService.listServers("g/1")).resolves.toEqual(
      reponse,
    );
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/games/g%2F1/servers",
      "g/1",
    );
  });

  it("listTemplates interroge le catalogue de templates", async () => {
    await nexusGamesService.listTemplates("g1");
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/games/g1/templates",
      "g1",
    );
  });

  it.each([
    ["start", "start"],
    ["stop", "stop"],
    ["restart", "restart"],
    ["revealIp", "reveal-ip"],
  ] as const)(
    "%s envoie l'action sur le serveur (encodage inclus)",
    async (methode, segment) => {
      await nexusGamesService[methode]("g1", "ser/ver");

      expect(mocks.nexusPost).toHaveBeenCalledWith(
        `/api/games/servers/ser%2Fver/${segment}`,
        "g1",
      );
    },
  );

  it("schedule programme la revelation avec date de cloture optionnelle", async () => {
    await nexusGamesService.schedule("g1", "s1", "2026-09-01T00:00:00Z");
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/games/servers/s1/schedule",
      "g1",
      { reveal_at: "2026-09-01T00:00:00Z", closes_at: null },
    );

    await nexusGamesService.schedule("g1", "s1", "r", "c");
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/games/servers/s1/schedule",
      "g1",
      { reveal_at: "r", closes_at: "c" },
    );
  });

  it("setRevealSchedule efface la revelation avec null", async () => {
    await nexusGamesService.setRevealSchedule("g1", "s1", null);
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/games/servers/s1/reveal-schedule",
      "g1",
      { reveal_at: null },
    );

    await nexusGamesService.setRevealSchedule("g1", "s1", "2026-09-01T00:00:00Z");
    expect(mocks.nexusPost).toHaveBeenLastCalledWith(
      "/api/games/servers/s1/reveal-schedule",
      "g1",
      { reveal_at: "2026-09-01T00:00:00Z" },
    );
  });

  it("getServer lit le detail + configuration effective", async () => {
    const reponse = { server: {}, config: {} };
    mocks.nexusGet.mockResolvedValue(reponse);
    await expect(nexusGamesService.getServer("g1", "s/1")).resolves.toEqual(
      reponse,
    );
    expect(mocks.nexusGet).toHaveBeenCalledWith("/api/games/servers/s%2F1", "g1");
  });

  it("create envoie le payload de creation", async () => {
    const payload = { template_slug: "mc", name: "N", owner_user_id: "u" };
    await nexusGamesService.create("g/1", payload);
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/games/g%2F1/servers",
      "g/1",
      payload,
    );
  });

  it("logs demande un nombre de lignes par defaut puis explicite", async () => {
    await nexusGamesService.logs("g1", "s1");
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/games/servers/s1/logs?lines=200",
      "g1",
    );

    await nexusGamesService.logs("g1", "s1", 5);
    expect(mocks.nexusGet).toHaveBeenLastCalledWith(
      "/api/games/servers/s1/logs?lines=5",
      "g1",
    );
  });

  it("stats lit les mesures en direct", async () => {
    await nexusGamesService.stats("g1", "s1");
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/games/servers/s1/stats",
      "g1",
    );
  });

  it("updateConfig enregistre les overrides dans le corps", async () => {
    await nexusGamesService.updateConfig("g1", "s1", { motd: "bonjour" });
    expect(mocks.nexusPut).toHaveBeenCalledWith(
      "/api/games/servers/s1/config",
      "g1",
      { config: { motd: "bonjour" } },
    );
  });

  it("rcon envoie la commande brute", async () => {
    await nexusGamesService.rcon("g1", "s1", "say hello");
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/games/servers/s1/command",
      "g1",
      { command: "say hello" },
    );
  });

  it("updateResources ajuste memoire et cœurs (null conserve l'auto)", async () => {
    await nexusGamesService.updateResources("g1", "s1", 4096, null);
    expect(mocks.nexusPut).toHaveBeenCalledWith(
      "/api/games/servers/s1/resources",
      "g1",
      { memory_mb: 4096, cpu_limit: null },
    );

    await nexusGamesService.updateResources("g1", "s1", 2048, 2);
    expect(mocks.nexusPut).toHaveBeenLastCalledWith(
      "/api/games/servers/s1/resources",
      "g1",
      { memory_mb: 2048, cpu_limit: 2 },
    );
  });

  it("getScheduleRanges lit les plages d'ouverture", async () => {
    await nexusGamesService.getScheduleRanges("g1", "s1");
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/games/servers/s1/schedule-ranges",
      "g1",
    );
  });

  it("saveScheduleRanges envoie le calendrier complet", async () => {
    const planning = {
      enabled: true,
      mode: "ranges" as const,
      timezone: "Europe/Paris",
      ranges: [{ start_minute: 360, end_minute: 1200 }],
      warn_minutes: 5,
      restart_interval_hours: null,
      restart_anchor_minute: 0,
    };
    await nexusGamesService.saveScheduleRanges("g1", "s1", planning);
    expect(mocks.nexusPut).toHaveBeenCalledWith(
      "/api/games/servers/s1/schedule-ranges",
      "g1",
      planning,
    );

    const permanence = { ...planning, mode: "restart" as const };
    await nexusGamesService.saveScheduleRanges("g1", "s1", permanence);
    expect(mocks.nexusPut).toHaveBeenLastCalledWith(
      "/api/games/servers/s1/schedule-ranges",
      "g1",
      permanence,
    );
  });

  it("getAlertSettings lit les seuils de supervision", async () => {
    await nexusGamesService.getAlertSettings("g1", "s1");
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/games/servers/s1/alerts",
      "g1",
    );
  });

  it("saveAlertSettings envoie les seuils (webhook optionnel)", async () => {
    await nexusGamesService.saveAlertSettings("g1", "s1", {
      cpu_threshold: 80,
      ram_threshold: 90,
      latency_threshold_ms: 500,
    });
    expect(mocks.nexusPut).toHaveBeenCalledWith(
      "/api/games/servers/s1/alerts",
      "g1",
      { cpu_threshold: 80, ram_threshold: 90, latency_threshold_ms: 500 },
    );

    await nexusGamesService.saveAlertSettings("g1", "s1", {
      webhook_url: "https://hooks.example.com/x",
      cpu_threshold: 70,
      ram_threshold: 80,
      latency_threshold_ms: 300,
    });
    expect(mocks.nexusPut).toHaveBeenLastCalledWith(
      "/api/games/servers/s1/alerts",
      "g1",
      {
        webhook_url: "https://hooks.example.com/x",
        cpu_threshold: 70,
        ram_threshold: 80,
        latency_threshold_ms: 300,
      },
    );
  });

  it("deleteAlertSettings arrete la surveillance", async () => {
    await nexusGamesService.deleteAlertSettings("g1", "s1");
    expect(mocks.nexusDelete).toHaveBeenCalledWith(
      "/api/games/servers/s1/alerts",
      "g1",
    );
  });

  it("commands lit le catalogue d'administration du jeu", async () => {
    await nexusGamesService.commands("g1", "s1");
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/games/servers/s1/commands",
      "g1",
    );
  });

  it("runCommand envoie la cle et les parametres (jamais le gabarit)", async () => {
    await nexusGamesService.runCommand("g1", "s/1", "kick player", { id: "42" });
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/games/servers/s%2F1/commands/kick%20player",
      "g1",
      { params: { id: "42" } },
    );
  });

  it("onlinePlayers interroge le jeu en direct", async () => {
    await nexusGamesService.onlinePlayers("g1", "s1");
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/games/servers/s1/players/online",
      "g1",
    );
  });

  it("sessions sans options demande la route nue", async () => {
    await nexusGamesService.sessions("g1", "s1");
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/games/servers/s1/sessions",
      "g1",
    );
  });

  it("sessions serialise limit/offset quand fournis (zero inclus)", async () => {
    await nexusGamesService.sessions("g1", "s1", { limit: 25, offset: 0 });
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/games/servers/s1/sessions?limit=25&offset=0",
      "g1",
    );

    await nexusGamesService.sessions("g1", "s1", { limit: 10 });
    expect(mocks.nexusGet).toHaveBeenLastCalledWith(
      "/api/games/servers/s1/sessions?limit=10",
      "g1",
    );

    await nexusGamesService.sessions("g1", "s1", { offset: 50 });
    expect(mocks.nexusGet).toHaveBeenLastCalledWith(
      "/api/games/servers/s1/sessions?offset=50",
      "g1",
    );
  });

  it("perfHistory envoie la plage et le pas optionnel", async () => {
    await nexusGamesService.perfHistory("g1", "s1", 3600);
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/games/servers/s1/perf-history?range_secs=3600",
      "g1",
    );

    await nexusGamesService.perfHistory("g1", "s1", 7200, 60);
    expect(mocks.nexusGet).toHaveBeenLastCalledWith(
      "/api/games/servers/s1/perf-history?range_secs=7200&step_secs=60",
      "g1",
    );
  });

  it("remove supprime le serveur", async () => {
    await nexusGamesService.remove("g1", "s/1");
    expect(mocks.nexusDelete).toHaveBeenCalledWith(
      "/api/games/servers/s%2F1",
      "g1",
    );
  });

  it("propage les erreurs de la passerelle Nexus", async () => {
    const erreur = new Error("Nexus indisponible");
    mocks.nexusGet.mockRejectedValue(erreur);
    await expect(nexusGamesService.listServers("g1")).rejects.toBe(erreur);
  });
});
