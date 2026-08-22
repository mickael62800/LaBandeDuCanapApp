import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  opsGet: vi.fn(),
  opsPost: vi.fn(),
  opsDelete: vi.fn(),
}));

vi.mock("@/api/opsHttp", () => mocks);

import { serverSecurityService } from "./serverSecurityService";

describe("serverSecurityService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("topIps utilise la fenetre et le plafond par defaut", async () => {
    await serverSecurityService.topIps();
    expect(mocks.opsGet).toHaveBeenCalledWith(
      "/security/top-ips?window=1h&limit=20",
    );

    await serverSecurityService.topIps("7d", 5);
    expect(mocks.opsGet).toHaveBeenLastCalledWith(
      "/security/top-ips?window=7d&limit=5",
    );
  });

  it("authFailures utilise la fenetre et le plafond par defaut", async () => {
    await serverSecurityService.authFailures();
    expect(mocks.opsGet).toHaveBeenCalledWith(
      "/security/auth-failures?window=24h&limit=100",
    );

    await serverSecurityService.authFailures("7d", 50);
    expect(mocks.opsGet).toHaveBeenLastCalledWith(
      "/security/auth-failures?window=7d&limit=50",
    );
  });

  it.each([
    ["bannedIps", () => serverSecurityService.bannedIps(), "/security/banned-ips"],
    ["tlsCert", () => serverSecurityService.tlsCert(), "/security/tls-cert"],
    ["sshFailures", () => serverSecurityService.sshFailures(), "/security/ssh-failures"],
    ["diskTrend", () => serverSecurityService.diskTrend(), "/security/disk-trend"],
    ["connections", () => serverSecurityService.connections(), "/security/connections"],
    ["openPorts", () => serverSecurityService.openPorts(), "/security/open-ports"],
    ["trivy", () => serverSecurityService.trivy(), "/security/trivy"],
    [
      "fileIntegrity",
      () => serverSecurityService.fileIntegrity(),
      "/security/file-integrity",
    ],
    ["outbound", () => serverSecurityService.outbound(), "/security/outbound"],
    [
      "nginxSuspicious",
      () => serverSecurityService.nginxSuspicious(),
      "/security/nginx-suspicious",
    ],
    ["tlsErrors", () => serverSecurityService.tlsErrors(), "/security/tls-errors"],
    [
      "manualBans",
      () => serverSecurityService.manualBans(),
      "/security/manual-bans",
    ],
  ] as const)("%s interroge %s", async (_nom, appeler, route) => {
    await appeler();
    expect(mocks.opsGet).toHaveBeenCalledWith(route);
  });

  it("auditLogs serialise les filtres et garde le plafond par defaut", async () => {
    await serverSecurityService.auditLogs();
    expect(mocks.opsGet).toHaveBeenCalledWith(
      "/security/audit-logs?limit=100",
    );

    await serverSecurityService.auditLogs({
      guild_id: "g1",
      event_type_prefix: "ban",
      limit: 25,
    });
    expect(mocks.opsGet).toHaveBeenLastCalledWith(
      "/security/audit-logs?guild_id=g1&event_type_prefix=ban&limit=25",
    );

    await serverSecurityService.auditLogs({ guild_id: "g1" });
    expect(mocks.opsGet).toHaveBeenLastCalledWith(
      "/security/audit-logs?guild_id=g1&limit=100",
    );
  });

  it("trafficTrend envoie la fenetre et le pas de regroupement", async () => {
    await serverSecurityService.trafficTrend();
    expect(mocks.opsGet).toHaveBeenCalledWith(
      "/security/traffic-trend?window=24h&bucket_minutes=5",
    );

    await serverSecurityService.trafficTrend("6h", 10);
    expect(mocks.opsGet).toHaveBeenLastCalledWith(
      "/security/traffic-trend?window=6h&bucket_minutes=10",
    );
  });

  it("lastLogins honore le plafond par defaut puis explicite", async () => {
    await serverSecurityService.lastLogins();
    expect(mocks.opsGet).toHaveBeenCalledWith("/security/last-logins?limit=20");

    await serverSecurityService.lastLogins(5);
    expect(mocks.opsGet).toHaveBeenLastCalledWith(
      "/security/last-logins?limit=5",
    );
  });

  it("geoip encode la liste d'adresses en un seul parametre", async () => {
    await serverSecurityService.geoip(["1.2.3.4", "5.6.7.8"]);
    expect(mocks.opsGet).toHaveBeenCalledWith(
      "/security/geoip?ips=1.2.3.4%2C5.6.7.8",
    );

    await serverSecurityService.geoip(["9.9.9.9"]);
    expect(mocks.opsGet).toHaveBeenLastCalledWith("/security/geoip?ips=9.9.9.9");
  });

  it("containerChanges interroge la passerelle conteneurs", async () => {
    await serverSecurityService.containerChanges();
    expect(mocks.opsGet).toHaveBeenCalledWith("/containers/changes");
  });

  it("banIp envoie l'adresse et la raison optionnelle", async () => {
    await serverSecurityService.banIp("10.0.0.9");
    expect(mocks.opsPost).toHaveBeenCalledWith("/security/ban-ip", {
      ip: "10.0.0.9",
      reason: undefined,
    });

    await serverSecurityService.banIp("10.0.0.9", "scanner");
    expect(mocks.opsPost).toHaveBeenLastCalledWith("/security/ban-ip", {
      ip: "10.0.0.9",
      reason: "scanner",
    });
  });

  it("unbanIp envoie l'adresse et la raison optionnelle", async () => {
    await serverSecurityService.unbanIp("10.0.0.9");
    expect(mocks.opsPost).toHaveBeenCalledWith("/security/unban-ip", {
      ip: "10.0.0.9",
      reason: undefined,
    });

    await serverSecurityService.unbanIp("10.0.0.9", "faux positif");
    expect(mocks.opsPost).toHaveBeenLastCalledWith("/security/unban-ip", {
      ip: "10.0.0.9",
      reason: "faux positif",
    });
  });

  it("serverEvents serialise les filtres et garde le plafond par defaut", async () => {
    await serverSecurityService.serverEvents();
    expect(mocks.opsGet).toHaveBeenCalledWith(
      "/security/server-events?limit=100",
    );

    await serverSecurityService.serverEvents({
      action_prefix: "ban",
      severity: "critical",
      limit: 42,
    });
    expect(mocks.opsGet).toHaveBeenLastCalledWith(
      "/security/server-events?action_prefix=ban&severity=critical&limit=42",
    );

    await serverSecurityService.serverEvents({ action_prefix: "purge" });
    expect(mocks.opsGet).toHaveBeenLastCalledWith(
      "/security/server-events?action_prefix=purge&limit=100",
    );
  });

  it("cleanup sans options supprime la route nue", async () => {
    await serverSecurityService.cleanup();
    expect(mocks.opsDelete).toHaveBeenCalledWith("/security/cleanup");
  });

  it("cleanup serialise chaque option fournie (y compris false)", async () => {
    await serverSecurityService.cleanup({ older_than_days: 30 });
    expect(mocks.opsDelete).toHaveBeenLastCalledWith(
      "/security/cleanup?older_than_days=30",
    );

    await serverSecurityService.cleanup({
      include_api_logs: false, // false est une valeur valide : serialisee
      include_audit_logs: true,
    });
    expect(mocks.opsDelete).toHaveBeenLastCalledWith(
      "/security/cleanup?include_api_logs=false&include_audit_logs=true",
    );

    await serverSecurityService.cleanup({
      older_than_days: 7,
      include_server_events: true,
      include_successful_logins: false,
      include_manual_bans: true,
    });
    expect(mocks.opsDelete).toHaveBeenLastCalledWith(
      "/security/cleanup?older_than_days=7&include_server_events=true" +
        "&include_successful_logins=false&include_manual_bans=true",
    );
  });

  it("propage les erreurs de la passerelle Ops", async () => {
    const erreur = new Error("Ops indisponible");
    mocks.opsGet.mockRejectedValue(erreur);
    await expect(serverSecurityService.bannedIps()).rejects.toBe(erreur);
  });
});
