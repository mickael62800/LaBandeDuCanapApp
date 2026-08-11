import { defineStore } from "pinia";
import { ref } from "vue";
import { realtimeService } from "@/services/realtimeService";
import { listen, type UnlistenFn } from "@/api/events-api";

export const useRealtimeStore = defineStore("realtime", () => {
  const connected = ref(false);
  const wsUrl = ref("");
  let initialized = false;
  const unlisteners: UnlistenFn[] = [];

  async function connect() {
    try { await realtimeService.connect(); } catch { /* retry handled elsewhere */ }
  }

  function disconnect() {
    realtimeService.disconnect();
    connected.value = false;
  }

  function getStatus() {
    const s = realtimeService.status();
    connected.value = s.connected;
    wsUrl.value = s.url;
    return s;
  }

  async function init() {
    if (initialized) return;
    initialized = true;

    for (const u of unlisteners) u();
    unlisteners.length = 0;

    const u1 = await listen("ws:connected", (event) => {
      const payload = event.payload as { connected: boolean; url: string };
      connected.value = payload.connected;
      wsUrl.value = payload.url;
    });
    unlisteners.push(u1);

    const u2 = await listen("ws:disconnected", () => {
      connected.value = false;
    });
    unlisteners.push(u2);

    await connect();
  }

  function cleanup() {
    for (const u of unlisteners) u();
    unlisteners.length = 0;
    disconnect();
    initialized = false;
  }

  async function onEvent(eventType: string, callback: (data: unknown) => void): Promise<UnlistenFn> {
    const rawUnlisten = await listen(`ws:${eventType}`, (event) => callback(event.payload));
    let active = true;
    const managedUnlisten = () => {
      if (!active) return;
      active = false;
      rawUnlisten();
      const index = unlisteners.indexOf(managedUnlisten);
      if (index >= 0) unlisteners.splice(index, 1);
    };
    unlisteners.push(managedUnlisten);
    return managedUnlisten;
  }

  return { connected, wsUrl, init, connect, disconnect, cleanup, getStatus, onEvent };
});
