import {
  errorDetails,
  HttpError,
  HttpTimeoutError,
  messageFromErrorBody,
  type HttpErrorDetails,
} from "./httpError";

export const DEFAULT_HTTP_TIMEOUT_MS = 15_000;

export interface JsonRequestOptions {
  url: string;
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  headers?: () => Record<string, string>;
  body?: unknown;
  credentials?: RequestCredentials;
  signal?: AbortSignal;
  timeoutMs?: number;
  backend?: string;
  emptyStatuses?: ReadonlySet<number>;
  /** Statuts GET transitoires a rejouer. Par defaut : 503 uniquement. */
  retryStatuses?: ReadonlySet<number>;
  refreshSession?: () => Promise<boolean>;
  onUnauthorized?: () => void;
  makeError?: (message: string, details: HttpErrorDetails) => Error;
}

export interface JsonResponse<T> {
  data: T;
  response: Response;
}

function abortableDelay(ms: number, signal?: AbortSignal): Promise<void> {
  if (signal?.aborted) return Promise.reject(signal.reason);
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(resolve, ms);
    signal?.addEventListener("abort", () => {
      window.clearTimeout(timer);
      reject(signal.reason);
    }, { once: true });
  });
}

function retryDelay(response: Response, fallbackMs: number): number {
  const value = response.headers.get("Retry-After");
  if (!value) return fallbackMs;
  const seconds = Number(value);
  if (Number.isFinite(seconds)) return Math.min(Math.max(seconds * 1_000, 0), 5_000);
  const dateMs = Date.parse(value) - Date.now();
  return Number.isFinite(dateMs) ? Math.min(Math.max(dateMs, 0), 5_000) : fallbackMs;
}

/** fetch borné, avec composition du signal de l'appelant. */
export async function fetchWithTimeout(
  input: RequestInfo | URL,
  init: RequestInit = {},
  timeoutMs = DEFAULT_HTTP_TIMEOUT_MS,
): Promise<Response> {
  const controller = new AbortController();
  const sourceSignal = init.signal;
  let timedOut = false;
  const forwardAbort = () => controller.abort(sourceSignal?.reason);

  if (sourceSignal?.aborted) forwardAbort();
  else sourceSignal?.addEventListener("abort", forwardAbort, { once: true });

  const timer = window.setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, timeoutMs);

  try {
    return await fetch(input, { ...init, signal: controller.signal });
  } catch (error) {
    if (timedOut) throw new HttpTimeoutError(timeoutMs);
    throw error;
  } finally {
    window.clearTimeout(timer);
    sourceSignal?.removeEventListener("abort", forwardAbort);
  }
}

async function parseBody(response: Response): Promise<unknown> {
  const text = await response.text();
  if (!text) return undefined;
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return text;
  }
}

async function fetchWithRetry(options: JsonRequestOptions): Promise<Response> {
  const delays = [0, 500, 1_500];
  const retryStatuses = options.retryStatuses ?? new Set([503]);
  let response: Response | undefined;

  for (const fallbackDelay of delays) {
    if (response) {
      await response.body?.cancel().catch(() => undefined);
      await abortableDelay(retryDelay(response, fallbackDelay), options.signal);
    }
    response = await fetchWithTimeout(options.url, {
      method: options.method,
      headers: options.headers?.(),
      credentials: options.credentials ?? "include",
      body: options.body === undefined ? undefined : JSON.stringify(options.body),
      signal: options.signal,
    }, options.timeoutMs);
    if (options.method !== "GET" || !retryStatuses.has(response.status)) return response;
  }
  return response as Response;
}

/**
 * Transport JSON commun : retry GET/503, refresh 401 dédupliqué par l'appelant,
 * parsing unique et conservation de la Response pour les en-têtes.
 */
export async function requestJson<T>(
  options: JsonRequestOptions,
): Promise<JsonResponse<T>> {
  let response = await fetchWithRetry(options);

  if (response.status === 401 && options.refreshSession) {
    if (await options.refreshSession()) response = await fetchWithRetry(options);
  }

  const body = await parseBody(response);
  if (!response.ok) {
    if (response.status === 401) options.onUnauthorized?.();
    const details = errorDetails(response, body, options.backend);
    const message = messageFromErrorBody(response.status, body);
    throw options.makeError?.(message, details) ?? new HttpError(message, details);
  }

  const data = options.emptyStatuses?.has(response.status)
    ? undefined as T
    : body as T;
  return { data, response };
}
