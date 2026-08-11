// Connexion WebSocket singleton. Chaque frame est publiée sur le canal
// générique `ws:event` et sur son canal spécialisé `ws:<event>`.

import { emit } from "@/api/events";
import { getApiConfig } from "@/api/config";
import { tryRefreshSession } from "@/api/http";

const OPEN_TIMEOUT_MS = 10_000;
const MAX_RECONNECT_DELAY_MS = 30_000;

let ws: WebSocket | null = null;
let wsUrl = "";
let wsConnected = false;
let connectInFlight: Promise<void> | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectAttempt = 0;
let manualDisconnect = false;
let cancelPendingOpen: (() => void) | null = null;

/**
 * Construit uniquement l'adresse du gateway. Les credentials ne doivent
 * jamais faire partie d'une URL WebSocket : en production, le navigateur
 * joint automatiquement le cookie HttpOnly same-origin au handshake.
 */
function deriveGatewayWs(apiUrl: string): string {
  try {
    const url = new URL(apiUrl);
    const isProd = url.hostname !== "localhost" && url.hostname !== "127.0.0.1";
    const port = isProd
      ? (url.port || (url.protocol === "https:" ? "443" : "80"))
      : (url.port ? String(Number(url.port) + 1) : "3001");
    const scheme = url.protocol === "https:" ? "wss" : "ws";
    return `${scheme}://${url.hostname}:${port}/ws`;
  } catch {
    return "";
  }
}

function clearReconnectTimer(): void {
  if (reconnectTimer) clearTimeout(reconnectTimer);
  reconnectTimer = null;
}

function canReconnect(): boolean {
  return !manualDisconnect && navigator.onLine !== false && !document.hidden;
}

function scheduleReconnect(immediate = false): void {
  if (!canReconnect() || reconnectTimer || connectInFlight || wsConnected) return;
  const exponential = Math.min(1_000 * (2 ** reconnectAttempt), MAX_RECONNECT_DELAY_MS);
  const jittered = Math.round(exponential * (0.8 + Math.random() * 0.4));
  const delay = immediate ? 0 : jittered;
  reconnectAttempt += 1;
  emit("ws:reconnecting", { attempt: reconnectAttempt, delay });
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    void startConnection(true).catch(() => { /* la fermeture planifie la suite */ });
  }, delay);
}

function closeCurrentSocket(): void {
  cancelPendingOpen?.();
  cancelPendingOpen = null;
  const current = ws;
  ws = null;
  if (!current) return;
  current.onopen = null;
  current.onclose = null;
  current.onerror = null;
  current.onmessage = null;
  try { current.close(); } catch { /* déjà fermé */ }
}

async function openSocket(): Promise<void> {
  const cfg = getApiConfig();
  if (!cfg?.api_url) throw new Error("API not configured");

  const url = deriveGatewayWs(cfg.api_url);
  if (!url) throw new Error("Invalid WebSocket URL");
  wsUrl = url;
  closeCurrentSocket();

  await new Promise<void>((resolve, reject) => {
    let socket: WebSocket;
    let settled = false;
    let opened = false;
    let openTimer: ReturnType<typeof setTimeout> | null = null;
    try {
      socket = new WebSocket(url);
      ws = socket;
    } catch (error) {
      reject(error);
      return;
    }

    const settleResolve = () => {
      if (settled) return;
      settled = true;
      if (openTimer) clearTimeout(openTimer);
      if (cancelPendingOpen === cancel) cancelPendingOpen = null;
      resolve();
    };
    const settleReject = (error: Error) => {
      if (settled) return;
      settled = true;
      if (openTimer) clearTimeout(openTimer);
      if (cancelPendingOpen === cancel) cancelPendingOpen = null;
      reject(error);
    };
    const cancel = () => settleReject(new Error("WebSocket connection cancelled"));
    cancelPendingOpen = cancel;
    openTimer = setTimeout(() => {
      settleReject(new Error("WebSocket connection timeout"));
      try { socket.close(); } catch { /* ignore */ }
    }, OPEN_TIMEOUT_MS);

    socket.onopen = () => {
      if (ws !== socket) return;
      opened = true;
      wsConnected = true;
      reconnectAttempt = 0;
      emit("ws:connected", { connected: true, url: wsUrl });
      settleResolve();
    };
    socket.onclose = () => {
      if (ws !== socket) return;
      ws = null;
      wsConnected = false;
      emit("ws:disconnected", null);
      if (!opened) settleReject(new Error("WebSocket closed before opening"));
      scheduleReconnect();
    };
    socket.onerror = () => {
      if (!opened) settleReject(new Error("WebSocket connection failed"));
    };
    socket.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data as string) as { event?: unknown; data?: unknown };
        if (typeof message.event !== "string") return;
        const envelope = { event: message.event, data: message.data };
        emit("ws:event", envelope);
        emit(`ws:${message.event}`, message.data);
      } catch { /* frame invalide ignorée */ }
    };
  });
}

async function startConnection(isReconnect = false): Promise<void> {
  if (wsConnected || ws?.readyState === WebSocket.OPEN) return;
  if (connectInFlight) return connectInFlight;
  clearReconnectTimer();
  manualDisconnect = false;

  const pending = (async () => {
    if (isReconnect) await tryRefreshSession();
    await openSocket();
  })();
  connectInFlight = pending;
  const clear = () => {
    if (connectInFlight === pending) connectInFlight = null;
  };
  pending.then(clear, () => {
    clear();
    scheduleReconnect();
  });
  return pending;
}

export const realtimeService = {
  connect(): Promise<void> {
    return startConnection(false);
  },

  disconnect(): void {
    manualDisconnect = true;
    clearReconnectTimer();
    closeCurrentSocket();
    wsConnected = false;
    connectInFlight = null;
  },

  status(): { connected: boolean; url: string; reconnectAttempt: number } {
    return { connected: wsConnected, url: wsUrl, reconnectAttempt };
  },
};

window.addEventListener("online", () => scheduleReconnect(true));
document.addEventListener("visibilitychange", () => {
  if (!document.hidden) scheduleReconnect(true);
  else clearReconnectTimer();
});
