import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { on } from "@/api/events";
import { setApiConfig, setDiscordToken } from "@/api/config";
import { realtimeService } from "./realtimeService";

function memoryStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() { return values.size; },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    key: (index) => [...values.keys()][index] ?? null,
    removeItem: (key) => { values.delete(key); },
    setItem: (key, value) => { values.set(key, String(value)); },
  };
}

class FakeWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSED = 3;
  static instances: FakeWebSocket[] = [];

  readyState = FakeWebSocket.CONNECTING;
  onopen: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;

  constructor(public readonly url: string) {
    FakeWebSocket.instances.push(this);
  }

  open(): void {
    this.readyState = FakeWebSocket.OPEN;
    this.onopen?.(new Event("open"));
  }

  message(data: unknown): void {
    this.onmessage?.(new MessageEvent("message", { data: JSON.stringify(data) }));
  }

  serverClose(): void {
    this.readyState = FakeWebSocket.CLOSED;
    this.onclose?.(new CloseEvent("close"));
  }

  close(): void {
    this.serverClose();
  }
}

describe("realtimeService", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.stubGlobal("localStorage", memoryStorage());
    vi.stubGlobal("sessionStorage", memoryStorage());
    vi.stubGlobal("WebSocket", FakeWebSocket);
    FakeWebSocket.instances = [];
    setApiConfig({ api_url: "http://localhost:3000" });
    setDiscordToken("initial-token");
  });

  afterEach(() => {
    realtimeService.disconnect();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("résout à l'ouverture et publie les canaux générique et spécialisé", async () => {
    const generic: unknown[] = [];
    const specialized: unknown[] = [];
    const offGeneric = on("ws:event", ({ payload }) => generic.push(payload));
    const offSpecialized = on("ws:bot_heartbeat", ({ payload }) => specialized.push(payload));

    const connection = realtimeService.connect();
    const socket = FakeWebSocket.instances[0];
    expect(socket?.url).toBe("ws://localhost:3001/ws");
    expect(socket?.url).not.toContain("initial-token");
    socket?.open();
    await connection;
    socket?.message({ event: "bot_heartbeat", data: { alive: true } });

    expect(generic).toEqual([{ event: "bot_heartbeat", data: { alive: true } }]);
    expect(specialized).toEqual([{ alive: true }]);
    expect(realtimeService.status().connected).toBe(true);
    offGeneric();
    offSpecialized();
  });

  // Le SPA ne detient plus de cle API du tout (elle a quitte `ApiConfig`).
  // Ce test garde son sens : l'URL WebSocket ne doit porter AUCUN parametre,
  // une query string finissant dans les logs de tout intermediaire.
  it("n'ajoute aucun paramètre à l'URL WebSocket", async () => {
    setApiConfig({ api_url: "http://localhost:3000" });

    const connection = realtimeService.connect();
    const socket = FakeWebSocket.instances[0];
    expect(socket?.url).toBe("ws://localhost:3001/ws");
    expect(new URL(socket!.url).search).toBe("");
    socket?.open();
    await connection;
  });

  it("rejette quand la socket ferme avant son ouverture", async () => {
    const connection = realtimeService.connect();
    FakeWebSocket.instances[0]?.serverClose();
    await expect(connection).rejects.toThrow("before opening");
  });

  it("borne la promesse de connexion", async () => {
    vi.useFakeTimers();
    const connection = realtimeService.connect();
    const assertion = expect(connection).rejects.toThrow("timeout");
    await vi.advanceTimersByTimeAsync(10_000);
    await assertion;
  });

  it("rafraîchit la session avant une reconnexion sans exposer le token", async () => {
    vi.useFakeTimers();
    vi.spyOn(Math, "random").mockReturnValue(0);
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({
      token: "renewed-token",
      id: "1",
      username: "admin",
      is_superadmin: true,
    }), { status: 200, headers: { "Content-Type": "application/json" } })));

    const firstConnection = realtimeService.connect();
    FakeWebSocket.instances[0]?.open();
    await firstConnection;
    FakeWebSocket.instances[0]?.serverClose();

    await vi.advanceTimersByTimeAsync(800);
    expect(FakeWebSocket.instances).toHaveLength(2);
    expect(FakeWebSocket.instances[1]?.url).toBe("ws://localhost:3001/ws");
    expect(FakeWebSocket.instances[1]?.url).not.toContain("renewed-token");
    FakeWebSocket.instances[1]?.open();
    expect(realtimeService.status().connected).toBe(true);
  });
});
