import { getDiscordToken } from "./config";
import { handleUnauthorizedSession, tryRefreshSession } from "./http";
import { HttpError, type HttpErrorDetails } from "./httpError";
import { requestJson } from "./httpTransport";

export class BackendHttpError extends HttpError {
  constructor(message: string, details: HttpErrorDetails, name = "BackendHttpError") {
    super(message, details, name);
  }
}

interface BackendClientOptions {
  baseUrl: string;
  errorLabel: string;
  forbiddenMessage: string;
  unavailableMessage?: string;
  emptyStatuses?: readonly number[];
  makeError: (message: string, details: HttpErrorDetails) => Error;
}

export interface BackendRequestOptions {
  body?: unknown;
  headers?: Record<string, string>;
  signal?: AbortSignal;
  timeoutMs?: number;
}

/** Adaptateur métier d'un backend, posé sur le transport commun. */
export function createBackendClient(options: BackendClientOptions) {
  const emptyStatuses = new Set(options.emptyStatuses ?? [204]);

  return async function request<T>(
    method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE",
    path: string,
    requestOptions: BackendRequestOptions = {},
  ): Promise<T> {
    const { data } = await requestJson<T>({
      url: `${options.baseUrl}${path}`,
      method,
      headers: () => {
        const headers: Record<string, string> = {
          "Content-Type": "application/json",
          ...requestOptions.headers,
        };
        const token = getDiscordToken();
        if (token) headers["X-Discord-Token"] = token;
        return headers;
      },
      credentials: "include",
      body: requestOptions.body,
      signal: requestOptions.signal,
      timeoutMs: requestOptions.timeoutMs,
      backend: options.errorLabel,
      emptyStatuses,
      refreshSession: tryRefreshSession,
      onUnauthorized: handleUnauthorizedSession,
      makeError: (message, details) => {
        let visibleMessage = message;
        if (details.status === 401) {
          visibleMessage = "Session expirée — reconnecte-toi.";
        } else if (details.status === 403) {
          visibleMessage = options.forbiddenMessage;
        } else if (
          options.unavailableMessage &&
          (details.status === 502 || details.status === 503)
        ) {
          visibleMessage = options.unavailableMessage;
        } else if (message === `Erreur ${details.status}`) {
          visibleMessage = `Erreur ${options.errorLabel} (${details.status})`;
        }
        return options.makeError(visibleMessage, details);
      },
    });
    return data;
  };
}
