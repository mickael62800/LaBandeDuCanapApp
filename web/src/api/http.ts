// Client Sentinel authentifié. Le transport bas niveau est partagé avec les
// backends Nexus, Ops et Atrium via httpTransport.ts.

import {
  clearDiscordToken,
  getApiConfig,
  getDiscordToken,
  setDiscordToken,
  setDiscordUser,
} from "./config";
import { HttpError, type HttpErrorDetails } from "./httpError";
import { requestJson, type JsonResponse } from "./httpTransport";

const SESSION_TIMEOUT_MS = 5_000;

export interface HttpRequestOptions {
  signal?: AbortSignal;
  timeoutMs?: number;
}

export function apiBase(): string {
  const cfg = getApiConfig()?.api_url;
  if (cfg) {
    try {
      const url = new URL(cfg);
      if (url.protocol === "https:" || url.protocol === "http:") return cfg;
    } catch { /* URL malformée : fallback sûr. */ }
  }
  const env = import.meta.env.VITE_API_URL;
  if (env) return env;
  return import.meta.env.PROD ? "" : "http://localhost:3000";
}

function headers(): Record<string, string> {
  const result: Record<string, string> = { "Content-Type": "application/json" };
  // Plus d'`Authorization: Bearer` pose par le SPA : la seule identite que le
  // navigateur detient est le jeton Discord ci-dessous, et les secrets de
  // service sont injectes par nginx, cote serveur. Cf. `ApiConfig`.
  const token = getDiscordToken();
  if (token) result["X-Discord-Token"] = token;
  return result;
}

let refreshInFlight: Promise<boolean> | null = null;

/** Ré-émet un token via le cookie HttpOnly et déduplique les appels concurrents. */
export function tryRefreshSession(): Promise<boolean> {
  if (refreshInFlight) return refreshInFlight;
  refreshInFlight = (async () => {
    try {
      const { data } = await requestJson<{
        token?: string;
        id: string;
        username: string;
        global_name?: string | null;
        avatar?: string | null;
        is_superadmin?: boolean;
      }>({
        url: `${apiBase()}/auth/refresh`,
        method: "POST",
        headers: () => ({ "Content-Type": "application/json" }),
        credentials: "include",
        timeoutMs: SESSION_TIMEOUT_MS,
      });
      if (!data?.token) return false;
      setDiscordToken(data.token);
      setDiscordUser({
        id: data.id,
        username: data.username,
        global_name: data.global_name ?? null,
        avatar: data.avatar ?? null,
        is_superadmin: data.is_superadmin === true,
      });
      return true;
    } catch {
      return false;
    }
  })().finally(() => {
    refreshInFlight = null;
  });
  return refreshInFlight;
}

/** Supprime la session serveur et son cookie, en best-effort borné. */
export async function logoutSession(): Promise<void> {
  try {
    await requestJson({
      url: `${apiBase()}/auth/logout`,
      method: "POST",
      credentials: "include",
      timeoutMs: SESSION_TIMEOUT_MS,
      emptyStatuses: new Set([204]),
    });
  } catch { /* La purge locale reste prioritaire. */ }
}

/** Réaction commune à une session réellement perdue après tentative de refresh. */
export function handleUnauthorizedSession(): void {
  const path = window.location.pathname;
  // Le callback OAuth possède son propre cycle de vie : une ancienne requête
  // ne doit jamais effacer le token qu'il vient de recevoir.
  if (path.startsWith("/auth/")) return;

  clearDiscordToken();
  setDiscordUser(null);
  if (path !== "/login") window.location.href = "/login?expired=1";
}

function makeSentinelError(message: string, details: HttpErrorDetails): HttpError {
  const visibleMessage = details.status === 401
    ? "Unauthorized: session expired"
    : message;
  return new HttpError(visibleMessage, { ...details, backend: "Sentinel" });
}

async function request<T>(
  path: string,
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE",
  body?: unknown,
  control: HttpRequestOptions = {},
): Promise<JsonResponse<T>> {
  const isAuthRoute = path.startsWith("/auth/");
  return requestJson<T>({
    url: `${apiBase()}${path}`,
    method,
    headers,
    body,
    credentials: "include",
    signal: control.signal,
    timeoutMs: control.timeoutMs,
    backend: "Sentinel",
    refreshSession: isAuthRoute ? undefined : tryRefreshSession,
    onUnauthorized: isAuthRoute ? undefined : handleUnauthorizedSession,
    makeError: makeSentinelError,
  });
}

export async function httpGet<T>(path: string, control?: HttpRequestOptions): Promise<T> {
  return (await request<T>(path, "GET", undefined, control)).data;
}

export async function httpGetWithTotal<T>(
  path: string,
  control?: HttpRequestOptions,
): Promise<{ data: T; total: number }> {
  const { data, response } = await request<T>(path, "GET", undefined, control);
  const rawTotal = response.headers.get("X-Total-Count");
  const parsedTotal = rawTotal === null ? Number.NaN : Number(rawTotal);
  return {
    data,
    total: Number.isFinite(parsedTotal)
      ? parsedTotal
      : Array.isArray(data) ? data.length : 0,
  };
}

export async function httpPost<T>(
  path: string,
  body?: unknown,
  control?: HttpRequestOptions,
): Promise<T> {
  return (await request<T>(path, "POST", body, control)).data;
}

export async function httpPut<T>(
  path: string,
  body?: unknown,
  control?: HttpRequestOptions,
): Promise<T> {
  return (await request<T>(path, "PUT", body, control)).data;
}

export async function httpPatch<T>(
  path: string,
  body?: unknown,
  control?: HttpRequestOptions,
): Promise<T> {
  return (await request<T>(path, "PATCH", body, control)).data;
}

export async function httpDelete<T>(
  path: string,
  body?: unknown,
  control?: HttpRequestOptions,
): Promise<T> {
  return (await request<T>(path, "DELETE", body, control)).data;
}

export { HttpError } from "./httpError";
